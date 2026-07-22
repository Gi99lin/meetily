//! Offline speaker diarization: identifies how many distinct voices are in a
//! meeting recording and which segments each one spoke, via ML clustering
//! rather than the mic/system channel heuristic in `audio::pipeline`.
//!
//! Gated behind the `diarization` Cargo feature (see Cargo.toml) since it
//! pulls in a second, independent onnxruntime build via the `sherpa-onnx`
//! crate. Commands are always registered; when the feature is off they
//! return a clear "not available in this build" error instead of failing
//! to compile or being silently missing.

pub mod commands;
pub mod models;

#[cfg(feature = "diarization")]
pub mod engine;

pub use commands::*;
