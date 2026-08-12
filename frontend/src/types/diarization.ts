// Types for the offline speaker diarization Tauri commands.
// See frontend/src-tauri/src/diarization/ for the Rust side.

export interface DiarizationModelsStatus {
  segmentation_ready: boolean;
  embedding_ready: boolean;
}

export interface SpeakerDiarizationResult {
  meeting_id: string;
  num_speakers: number;
  segments_updated: number;
}

export type DiarizationStage =
  | 'queued'
  | 'decoding'
  | 'processing'
  | 'saving'
  | 'complete'
  | 'failed';

export interface DiarizationStatusEvent {
  meeting_id: string;
  stage: DiarizationStage;
  error?: string;
}

export interface MeetingSpeaker {
  meeting_id: string;
  speaker_key: string;
  display_name: string;
}

export function areDiarizationModelsReady(status: DiarizationModelsStatus): boolean {
  return status.segmentation_ready && status.embedding_ready;
}
