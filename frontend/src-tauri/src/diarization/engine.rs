//! Wraps sherpa-onnx's offline speaker diarization (pyannote segmentation +
//! WeSpeaker embedding + clustering) and attributes the resulting speaker
//! clusters back onto Whisper/Parakeet transcript segments by time overlap.
//!
//! sherpa-onnx runs its own independent speech segmentation internally — its
//! segment boundaries do NOT line up with Whisper's transcript segments, so
//! the two have to be reconciled after the fact via overlap, the same way
//! `audio::pipeline::AudioPipeline::dominant_speaker_for_range` reconciles
//! per-window mic/system energy against VAD segment boundaries.

use anyhow::{anyhow, Result};
use sherpa_onnx::{
    FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
    OfflineSpeakerSegmentationModelConfig, OfflineSpeakerSegmentationPyannoteModelConfig,
    SpeakerEmbeddingExtractorConfig,
};
use std::path::Path;

/// One speaker-attributed time range from sherpa-onnx's diarization result.
#[derive(Debug, Clone, Copy)]
pub struct DiarizedSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    /// 0-based cluster index assigned by sherpa-onnx; formatted elsewhere as
    /// e.g. "speaker_00".
    pub speaker_index: i32,
}

pub struct DiarizationEngine {
    inner: OfflineSpeakerDiarization,
}

impl DiarizationEngine {
    pub fn new(segmentation_model: &Path, embedding_model: &Path) -> Result<Self> {
        let config = OfflineSpeakerDiarizationConfig {
            segmentation: OfflineSpeakerSegmentationModelConfig {
                pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                    model: Some(segmentation_model.to_string_lossy().to_string()),
                },
                ..Default::default()
            },
            embedding: SpeakerEmbeddingExtractorConfig {
                model: Some(embedding_model.to_string_lossy().to_string()),
                ..Default::default()
            },
            // -1 = auto-detect speaker count via `threshold` (default 0.5),
            // since we don't know how many people are in a meeting upfront.
            clustering: FastClusteringConfig {
                num_clusters: -1,
                ..Default::default()
            },
            ..Default::default()
        };

        let inner = OfflineSpeakerDiarization::create(&config).ok_or_else(|| {
            anyhow!(
                "Failed to initialize offline speaker diarization (segmentation model: {}, embedding model: {})",
                segmentation_model.display(),
                embedding_model.display()
            )
        })?;

        Ok(Self { inner })
    }

    /// Sample rate (Hz) the segmentation model expects. Callers must resample
    /// audio to this rate before calling [`Self::diarize`].
    pub fn expected_sample_rate(&self) -> i32 {
        self.inner.sample_rate()
    }

    /// Run diarization on a full mono waveform already at `expected_sample_rate()`.
    pub fn diarize(&self, samples: &[f32]) -> Result<Vec<DiarizedSegment>> {
        let result = self
            .inner
            .process(samples)
            .ok_or_else(|| anyhow!("Speaker diarization failed to process the audio"))?;

        log::info!(
            "Diarization found {} speaker(s) across {} segment(s)",
            result.num_speakers(),
            result.num_segments()
        );

        Ok(result
            .sort_by_start_time()
            .into_iter()
            .map(|s| DiarizedSegment {
                start_seconds: s.start as f64,
                end_seconds: s.end as f64,
                speaker_index: s.speaker,
            })
            .collect())
    }
}

/// Format a 0-based cluster index the way it's stored in `transcripts.speaker`.
pub fn speaker_key(speaker_index: i32) -> String {
    format!("speaker_{:02}", speaker_index)
}

/// For each `(start, end)` time range, find which diarized segment overlaps
/// it the most (by overlap duration) and return that segment's speaker key.
/// Returns `None` for ranges with no overlapping diarized speech at all.
pub fn dominant_speaker_key_for_range(
    diarized: &[DiarizedSegment],
    start_seconds: f64,
    end_seconds: f64,
) -> Option<String> {
    diarized
        .iter()
        .map(|seg| {
            let overlap = seg.end_seconds.min(end_seconds) - seg.start_seconds.max(start_seconds);
            (overlap, seg.speaker_index)
        })
        .filter(|(overlap, _)| *overlap > 0.0)
        .max_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, speaker_index)| speaker_key(speaker_index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speaker_key_formatting() {
        assert_eq!(speaker_key(0), "speaker_00");
        assert_eq!(speaker_key(1), "speaker_01");
        assert_eq!(speaker_key(12), "speaker_12");
    }

    fn segment(start: f64, end: f64, speaker: i32) -> DiarizedSegment {
        DiarizedSegment { start_seconds: start, end_seconds: end, speaker_index: speaker }
    }

    #[test]
    fn test_dominant_speaker_key_picks_max_overlap() {
        let diarized = vec![segment(0.0, 5.0, 0), segment(5.0, 12.0, 1)];
        // Transcript segment [4, 8] overlaps speaker 0 by 1s and speaker 1 by 3s.
        assert_eq!(
            dominant_speaker_key_for_range(&diarized, 4.0, 8.0),
            Some("speaker_01".to_string())
        );
    }

    #[test]
    fn test_dominant_speaker_key_no_overlap_is_none() {
        let diarized = vec![segment(0.0, 5.0, 0)];
        assert_eq!(dominant_speaker_key_for_range(&diarized, 10.0, 12.0), None);
    }

    #[test]
    fn test_dominant_speaker_key_empty_diarization_is_none() {
        assert_eq!(dominant_speaker_key_for_range(&[], 0.0, 5.0), None);
    }

    #[test]
    fn test_dominant_speaker_key_exact_tie_picks_one_consistently() {
        // Two speakers with identical overlap — max_by picks the later one in
        // iteration order on ties, which is fine as long as it's deterministic.
        let diarized = vec![segment(0.0, 2.0, 0), segment(2.0, 4.0, 1)];
        let result = dominant_speaker_key_for_range(&diarized, 1.0, 3.0);
        assert!(result == Some("speaker_00".to_string()) || result == Some("speaker_01".to_string()));
    }
}
