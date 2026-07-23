// Tauri commands for offline speaker diarization.
//
// Every command below is always registered/compiled, regardless of the
// `diarization` Cargo feature. When the feature is off, the "real" bodies
// are replaced by a stub returning NOT_AVAILABLE_MSG, so the frontend gets
// a clear, actionable error instead of a missing-command failure.

use crate::database::repositories::speaker::{MeetingSpeaker, SpeakersRepository};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use super::models::{self, DiarizationModelsStatus};

#[allow(dead_code)] // only read by the stub bodies compiled when the `diarization` feature is off
const NOT_AVAILABLE_MSG: &str =
    "Speaker diarization is not available in this build. Rebuild with `--features diarization` to enable it.";

fn models_dir<R: Runtime>(app: &AppHandle<R>) -> std::path::PathBuf {
    app.path()
        .app_data_dir()
        .expect("Failed to get app data dir")
        .join("models")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerDiarizationResult {
    pub meeting_id: String,
    pub num_speakers: i32,
    pub segments_updated: usize,
}

/// Cheap, feature-independent check of whether both models are downloaded.
#[tauri::command]
pub async fn get_diarization_models_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<DiarizationModelsStatus, String> {
    Ok(models::check_models_status(&models_dir(&app)))
}

/// Named speakers for a meeting (renamed "speaker_00" -> "Anna" entries).
/// Keys with no row here yet just aren't renamed; that's not an error.
#[tauri::command]
pub async fn get_meeting_speaker_names(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<MeetingSpeaker>, String> {
    SpeakersRepository::get_for_meeting(state.db_manager.pool(), &meeting_id)
        .await
        .map_err(|e| e.to_string())
}

/// Whether a diarization run is currently in progress for this meeting.
/// Lets the frontend restore the correct button state after navigating
/// away and back — the actual Tokio task keeps running server-side even
/// though any component-local "isDiarizing" state resets on unmount.
#[tauri::command]
pub async fn is_diarization_running(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<bool, String> {
    Ok(state
        .diarizing_meetings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains(&meeting_id))
}

#[tauri::command]
pub async fn rename_meeting_speaker(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    speaker_key: String,
    display_name: String,
) -> Result<(), String> {
    if display_name.trim().is_empty() {
        return Err("Display name cannot be empty".to_string());
    }
    SpeakersRepository::rename(state.db_manager.pool(), &meeting_id, &speaker_key, &display_name)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(feature = "diarization")]
#[tauri::command]
pub async fn download_diarization_models<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let dir = models_dir(&app);
    let app_for_progress = app.clone();
    let progress: models::ProgressCallback = Box::new(move |stage, pct| {
        use tauri::Emitter;
        let _ = app_for_progress.emit(
            "diarization-download-progress",
            serde_json::json!({ "stage": stage, "progress": pct }),
        );
    });

    models::ensure_models_downloaded(&dir, Some(&progress))
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(feature = "diarization"))]
#[tauri::command]
pub async fn download_diarization_models<R: Runtime>(_app: AppHandle<R>) -> Result<(), String> {
    Err(NOT_AVAILABLE_MSG.to_string())
}

/// Run offline speaker diarization for a meeting's recording, and update
/// every transcript segment that isn't already tagged "mic" (i.e. the
/// caller's own mic-channel segments from live-recording Phase 0 tagging
/// are left as "You" — only segments tagged "system" or untagged get a
/// refined "speaker_NN" label from ML clustering).
#[cfg(feature = "diarization")]
#[tauri::command]
pub async fn run_speaker_diarization<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<SpeakerDiarizationResult, String> {
    {
        let mut in_progress = state
            .diarizing_meetings
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if !in_progress.insert(meeting_id.clone()) {
            return Err(
                "Speaker detection is already running for this meeting.".to_string(),
            );
        }
    }

    let result = run_speaker_diarization_inner(&app, &state, meeting_id.clone()).await;

    state
        .diarizing_meetings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&meeting_id);

    result
}

#[cfg(feature = "diarization")]
async fn run_speaker_diarization_inner<R: Runtime>(
    app: &AppHandle<R>,
    state: &tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<SpeakerDiarizationResult, String> {
    use crate::audio::decoder::decode_audio_file;
    use crate::audio::retranscription::find_audio_file;
    use crate::database::repositories::meeting::MeetingsRepository;
    use crate::database::repositories::transcript::TranscriptsRepository;
    use super::engine::{dominant_speaker_key_for_range, DiarizationEngine};

    let dir = models_dir(app);
    let status = models::check_models_status(&dir);
    if !status.all_ready() {
        return Err(
            "Speaker diarization models aren't downloaded yet. Download them first.".to_string(),
        );
    }

    let pool = state.db_manager.pool().clone();

    let meeting = MeetingsRepository::get_meeting_metadata(&pool, &meeting_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting {} not found", meeting_id))?;

    let folder_path = meeting
        .folder_path
        .ok_or_else(|| "Meeting has no recording folder to diarize".to_string())?;

    let audio_path = find_audio_file(std::path::Path::new(&folder_path))
        .map_err(|e| format!("Could not find meeting audio: {}", e))?;

    let transcripts = TranscriptsRepository::get_all_for_meeting(&pool, &meeting_id)
        .await
        .map_err(|e| e.to_string())?;

    let segmentation_path = models::segmentation_model_path(&dir);
    let embedding_path = models::embedding_model_path(&dir);

    // Both audio decode/resample and ONNX inference are CPU-bound and
    // synchronous; run them off the async runtime so the Tauri event loop
    // (and this command's own long await) doesn't stall other work.
    let diarized = tokio::task::spawn_blocking(move || -> Result<_, String> {
        let decoded = decode_audio_file(&audio_path).map_err(|e| e.to_string())?;
        let samples = decoded.to_whisper_format(); // 16kHz mono f32

        let engine = DiarizationEngine::new(&segmentation_path, &embedding_path)
            .map_err(|e| e.to_string())?;

        let expected_rate = engine.expected_sample_rate();
        if expected_rate != 16000 {
            log::warn!(
                "Diarization segmentation model expects {}Hz, but audio was resampled to 16kHz; \
                 proceeding anyway since all verified sherpa-onnx segmentation/embedding models are 16kHz",
                expected_rate
            );
        }

        engine.diarize(&samples).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Diarization task panicked: {}", e))??;

    let num_speakers = diarized
        .iter()
        .map(|s| s.speaker_index)
        .collect::<std::collections::HashSet<_>>()
        .len() as i32;

    let mut segments_updated = 0usize;
    for transcript in &transcripts {
        if transcript.speaker.as_deref() == Some("mic") {
            continue; // Keep the caller's own high-confidence "You" tag as-is.
        }

        let (Some(start), Some(end)) = (transcript.audio_start_time, transcript.audio_end_time)
        else {
            continue;
        };

        if let Some(new_speaker) = dominant_speaker_key_for_range(&diarized, start, end) {
            if transcript.speaker.as_deref() != Some(new_speaker.as_str()) {
                TranscriptsRepository::update_speaker(&pool, &transcript.id, Some(new_speaker.as_str()))
                    .await
                    .map_err(|e| e.to_string())?;
                segments_updated += 1;
            }
        }
    }

    Ok(SpeakerDiarizationResult {
        meeting_id,
        num_speakers,
        segments_updated,
    })
}

#[cfg(not(feature = "diarization"))]
#[tauri::command]
pub async fn run_speaker_diarization<R: Runtime>(
    _app: AppHandle<R>,
    _state: tauri::State<'_, AppState>,
    _meeting_id: String,
) -> Result<SpeakerDiarizationResult, String> {
    Err(NOT_AVAILABLE_MSG.to_string())
}
