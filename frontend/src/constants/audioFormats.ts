/**
 * Supported audio/video file extensions for import and retranscription.
 * IMPORTANT: Keep in sync with Rust constant in src-tauri/src/audio/constants.rs
 *
 * Includes:
 * - Native formats: MP4, M4V, M4A, WAV, MP3, FLAC, OGG, AAC
 * - FFmpeg-backed: MKV, WebM, WMA, MOV, AVI, FLV, TS, MPG/MPEG, WMV, 3GP
 *
 * Video files are demuxed to their audio track only; no video is imported.
 */
export const AUDIO_EXTENSIONS = [
  'mp4', 'm4v', 'm4a', 'wav', 'mp3', 'flac', 'ogg', 'aac',
  'mkv', 'webm', 'wma', 'mov', 'avi', 'flv', 'ts', 'mpg', 'mpeg', 'wmv', '3gp',
] as const;

export type AudioExtension = typeof AUDIO_EXTENSIONS[number];

export const isAudioExtension = (ext: string): ext is AudioExtension =>{
  return (AUDIO_EXTENSIONS as readonly string[]).includes(ext);
}

/**
 * Human-readable format names for display
 */
export const AUDIO_FORMAT_DISPLAY_NAMES: Record<AudioExtension, string> = {
  mp4: 'MP4',
  m4v: 'M4V',
  m4a: 'M4A',
  wav: 'WAV',
  mp3: 'MP3',
  flac: 'FLAC',
  ogg: 'OGG',
  aac: 'AAC',
  mkv: 'MKV',
  webm: 'WebM',
  wma: 'WMA',
  mov: 'MOV',
  avi: 'AVI',
  flv: 'FLV',
  ts: 'TS',
  mpg: 'MPG',
  mpeg: 'MPEG',
  wmv: 'WMV',
  '3gp': '3GP',
};

/**
 * Get comma-separated list for UI display
 * Example: "MP4, M4A, WAV, MP3, FLAC, OGG, AAC, MKV, WebM, WMA"
 */
export function getAudioFormatsDisplayList(): string {
  return AUDIO_EXTENSIONS.map(ext => AUDIO_FORMAT_DISPLAY_NAMES[ext]).join(', ');
}
