/// Application configuration constants
///
/// Centralized definitions for default models and settings.
/// Used across database initialization, import, and retranscription.

/// Default Whisper model for transcription when no preference is configured.
/// This is the recommended balance of accuracy and speed.
pub const DEFAULT_WHISPER_MODEL: &str = "large-v3-turbo";

/// Default Parakeet model for transcription when no preference is configured.
/// This is the quantized version optimized for speed.
pub const DEFAULT_PARAKEET_MODEL: &str = "parakeet-tdt-0.6b-v3-int8";

/// Whisper model catalog with metadata for all supported models.
/// Used by both WhisperEngine::discover_models() and discover_models_standalone().
///
/// Format: (name, filename, size_mb, accuracy, speed, description)
pub const WHISPER_MODEL_CATALOG: &[(&str, &str, u32, &str, &str, &str)] = &[
    // Standard f16 models (full precision, multilingual)
    ("tiny", "ggml-tiny.bin", 74, "Decent", "Very Fast", "Fastest processing, good for real-time use"),
    ("base", "ggml-base.bin", 142, "Good", "Fast", "Good balance of speed and accuracy"),
    ("small", "ggml-small.bin", 466, "Good", "Medium", "Better accuracy, moderate speed"),
    ("medium", "ggml-medium.bin", 1463, "High", "Slow", "High accuracy for professional use"),
    ("large-v3-turbo", "ggml-large-v3-turbo.bin", 1549, "High", "Medium", "Best accuracy with improved speed"),
    ("large-v3", "ggml-large-v3.bin", 2951, "High", "Slow", "Most Accurate, latest large model"),
    ("large-v2", "ggml-large-v2.bin", 2951, "High", "Slow", "Multilingual, previous-generation large model, for compatibility/comparison"),

    // English-only f16 models (smaller & slightly more accurate than multilingual for English-only audio)
    ("tiny.en", "ggml-tiny.en.bin", 74, "Decent", "Very Fast", "English-only, fastest processing"),
    ("base.en", "ggml-base.en.bin", 141, "Good", "Fast", "English-only, good balance of speed and accuracy"),
    ("small.en", "ggml-small.en.bin", 466, "Good", "Medium", "English-only, better accuracy, moderate speed"),
    ("medium.en", "ggml-medium.en.bin", 1463, "High", "Slow", "English-only, high accuracy for professional use"),

    // Q5_1 quantized models (balanced speed/accuracy, slightly better quality than Q5_0)
    ("tiny-q5_1", "ggml-tiny-q5_1.bin", 31, "Decent", "Very Fast", "Quantized tiny model, ~50% faster processing"),
    ("base-q5_1", "ggml-base-q5_1.bin", 57, "Good", "Fast", "Quantized base model, good speed/accuracy balance"),
    ("small-q5_1", "ggml-small-q5_1.bin", 181, "Good", "Fast", "Quantized small model, faster than f16 version"),
    ("tiny.en-q5_1", "ggml-tiny.en-q5_1.bin", 31, "Decent", "Very Fast", "English-only quantized tiny model"),
    ("base.en-q5_1", "ggml-base.en-q5_1.bin", 57, "Good", "Fast", "English-only quantized base model"),
    ("small.en-q5_1", "ggml-small.en-q5_1.bin", 181, "Good", "Fast", "English-only quantized small model"),

    // Q5_0 quantized models (balanced speed/accuracy)
    ("medium-q5_0", "ggml-medium-q5_0.bin", 514, "High", "Medium", "Quantized medium model, professional quality"),
    ("large-v3-turbo-q5_0", "ggml-large-v3-turbo-q5_0.bin", 547, "High", "Medium", "Quantized large model, best balance"),
    ("large-v3-q5_0", "ggml-large-v3-q5_0.bin", 1031, "High", "Slow", "Quantized large model, high accuracy"),
    ("medium.en-q5_0", "ggml-medium.en-q5_0.bin", 514, "High", "Medium", "English-only quantized medium model"),

    // Q8_0 quantized models (near-lossless, larger than Q5 but closer to f16 accuracy)
    ("small-q8_0", "ggml-small-q8_0.bin", 252, "Good", "Medium", "Near-lossless quantized small model"),
    ("medium-q8_0", "ggml-medium-q8_0.bin", 785, "High", "Medium", "Near-lossless quantized medium model"),
    ("large-v3-turbo-q8_0", "ggml-large-v3-turbo-q8_0.bin", 834, "High", "Medium", "Near-lossless quantized large turbo model"),

    // Distilled models (separate architecture, English-only, optimized for speed)
    ("distil-large-v3", "ggml-distil-large-v3.bin", 1449, "High", "Fast", "English-only distilled model, faster than large-v3 with comparable accuracy"),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// `WhisperEngine::load_model` resolves a model's on-disk path from the
    /// catalog's `filename` field (via the cached `ModelInfo`), while
    /// `WhisperEngine::download_model` independently computes the destination
    /// filename as `format!("ggml-{}.bin", name)`. If a catalog entry's
    /// `filename` ever drifted from that pattern, a freshly downloaded model
    /// would be saved under one name but looked up under another, and
    /// `discover_models()` would report it as permanently "Missing".
    #[test]
    fn catalog_filenames_match_download_naming_convention() {
        for &(name, filename, _, _, _, _) in WHISPER_MODEL_CATALOG {
            let expected = format!("ggml-{}.bin", name);
            assert_eq!(
                filename, expected,
                "catalog entry '{}' has filename '{}', but download_model() would save it as '{}'",
                name, filename, expected
            );
        }
    }

    #[test]
    fn catalog_has_no_duplicate_names() {
        let mut names: Vec<&str> = WHISPER_MODEL_CATALOG.iter().map(|&(name, ..)| name).collect();
        let original_len = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), original_len, "duplicate model name in WHISPER_MODEL_CATALOG");
    }
}
