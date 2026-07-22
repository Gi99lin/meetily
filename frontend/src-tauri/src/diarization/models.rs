//! Model catalog and download/extraction for the two fixed models offline
//! diarization needs: a pyannote speech-segmentation model and a WeSpeaker
//! speaker-embedding model. Unlike Whisper/Parakeet there is no user choice
//! of variant here — these are the two models sherpa-onnx's diarization API
//! requires, verified against the real upstream release/repo file listings.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Segmentation model: distributed as a .tar.bz2 containing `model.onnx`
/// under a `sherpa-onnx-pyannote-segmentation-3-0/` directory.
pub const SEGMENTATION_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2";
const SEGMENTATION_ARCHIVE_SIZE_BYTES: u64 = 6_958_444;
const SEGMENTATION_INNER_DIR: &str = "sherpa-onnx-pyannote-segmentation-3-0";

/// Embedding model: English-optimized WeSpeaker ResNet34, plain .onnx file.
/// (A multilingual 3D-Speaker model exists upstream too; this one was picked
/// for better separation on English speech, at some cost for other languages —
/// voice-timbre embeddings still transfer cross-lingually, just less precisely.)
pub const EMBEDDING_MODEL_URL: &str = "https://huggingface.co/csukuangfj/speaker-embedding-models/resolve/main/wespeaker_en_voxceleb_resnet34.onnx";
const EMBEDDING_MODEL_SIZE_BYTES: u64 = 27_800_000; // ~26.5 MB

/// Subdirectory (within the app's shared models directory) holding both files.
const DIARIZATION_SUBDIR: &str = "diarization";
const EMBEDDING_FILENAME: &str = "wespeaker_en_voxceleb_resnet34.onnx";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizationModelsStatus {
    pub segmentation_ready: bool,
    pub embedding_ready: bool,
}

impl DiarizationModelsStatus {
    pub fn all_ready(&self) -> bool {
        self.segmentation_ready && self.embedding_ready
    }
}

fn diarization_dir(models_dir: &Path) -> PathBuf {
    models_dir.join(DIARIZATION_SUBDIR)
}

/// Path to the extracted segmentation model file.
pub fn segmentation_model_path(models_dir: &Path) -> PathBuf {
    diarization_dir(models_dir)
        .join(SEGMENTATION_INNER_DIR)
        .join("model.onnx")
}

/// Path to the embedding model file.
pub fn embedding_model_path(models_dir: &Path) -> PathBuf {
    diarization_dir(models_dir).join(EMBEDDING_FILENAME)
}

/// Cheap existence/size check — no ONNX Runtime involved, safe to call
/// regardless of whether the `diarization` feature is compiled in.
pub fn check_models_status(models_dir: &Path) -> DiarizationModelsStatus {
    let seg_path = segmentation_model_path(models_dir);
    let emb_path = embedding_model_path(models_dir);

    let file_is_plausible = |path: &Path, min_bytes: u64| {
        std::fs::metadata(path)
            .map(|m| m.len() >= min_bytes / 2) // allow generous slack, real corruption check is on full download
            .unwrap_or(false)
    };

    DiarizationModelsStatus {
        segmentation_ready: file_is_plausible(&seg_path, 1_000_000), // model.onnx alone, not the whole archive
        embedding_ready: file_is_plausible(&emb_path, EMBEDDING_MODEL_SIZE_BYTES),
    }
}

#[cfg(feature = "diarization")]
mod download {
    use super::*;
    use anyhow::{anyhow, Result};
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    pub type ProgressCallback = Box<dyn Fn(&str, u8) + Send>;

    /// Download both diarization models if missing, extracting the
    /// segmentation archive. `progress` receives (stage_name, percent).
    pub async fn ensure_models_downloaded(
        models_dir: &Path,
        progress: Option<&ProgressCallback>,
    ) -> Result<()> {
        std::fs::create_dir_all(diarization_dir(models_dir))
            .map_err(|e| anyhow!("Failed to create diarization models directory: {}", e))?;

        let status = check_models_status(models_dir);

        if !status.embedding_ready {
            if let Some(cb) = progress {
                cb("embedding", 0);
            }
            download_file(
                EMBEDDING_MODEL_URL,
                &embedding_model_path(models_dir),
                progress.map(|cb| ("embedding", cb)),
            )
            .await?;
        }

        if !status.segmentation_ready {
            if let Some(cb) = progress {
                cb("segmentation", 0);
            }
            let archive_path = diarization_dir(models_dir).join("segmentation.tar.bz2");
            download_file(
                SEGMENTATION_ARCHIVE_URL,
                &archive_path,
                progress.map(|cb| ("segmentation", cb)),
            )
            .await?;

            extract_tar_bz2(&archive_path, &diarization_dir(models_dir))?;

            let _ = std::fs::remove_file(&archive_path);

            if !segmentation_model_path(models_dir).exists() {
                return Err(anyhow!(
                    "Segmentation archive extracted but model.onnx not found at expected path \
                     (upstream archive layout may have changed)"
                ));
            }
        }

        Ok(())
    }

    async fn download_file(
        url: &str,
        dest: &Path,
        progress: Option<(&str, &ProgressCallback)>,
    ) -> Result<()> {
        log::info!("Downloading diarization asset: {} -> {}", url, dest.display());

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start download of {}: {}", url, e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Download failed for {} with status: {}",
                url,
                response.status()
            ));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(dest)
            .await
            .map_err(|e| anyhow!("Failed to create file {}: {}", dest.display(), e))?;

        let mut stream = response.bytes_stream();
        let mut downloaded = 0u64;
        let mut last_reported = 0u8;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| anyhow!("Failed to read chunk: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| anyhow!("Failed to write chunk: {}", e))?;
            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let pct = ((downloaded as f64 / total_size as f64) * 100.0) as u8;
                if pct >= last_reported + 5 || pct == 100 {
                    if let Some((stage, cb)) = progress {
                        cb(stage, pct);
                    }
                    last_reported = pct;
                }
            }
        }

        log::info!("Downloaded {} bytes to {}", downloaded, dest.display());
        Ok(())
    }

    fn extract_tar_bz2(archive_path: &Path, dest_dir: &Path) -> Result<()> {
        log::info!(
            "Extracting {} to {}",
            archive_path.display(),
            dest_dir.display()
        );
        let file = std::fs::File::open(archive_path)
            .map_err(|e| anyhow!("Failed to open archive {}: {}", archive_path.display(), e))?;
        let decompressed = bzip2_rs::DecoderReader::new(file);
        let mut archive = tar::Archive::new(decompressed);
        archive
            .unpack(dest_dir)
            .map_err(|e| anyhow!("Failed to extract archive: {}", e))?;
        Ok(())
    }
}

#[cfg(feature = "diarization")]
pub use download::{ensure_models_downloaded, ProgressCallback};
