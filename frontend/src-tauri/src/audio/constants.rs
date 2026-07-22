/// Supported audio/video file extensions for import and retranscription.
///
/// Includes native Symphonia formats (MP4, M4V, M4A, WAV, MP3, FLAC, OGG, AAC)
/// and FFmpeg-backed formats (MKV, WebM, WMA, MOV, AVI, FLV, TS, MPG/MPEG, WMV, 3GP).
/// Video files are demuxed to their audio track only; no video is decoded or stored.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "m4a", "wav", "mp3", "flac", "ogg", "aac",
    "mkv", "webm", "wma", "mov", "avi", "flv", "ts", "mpg", "mpeg", "wmv", "3gp",
];
