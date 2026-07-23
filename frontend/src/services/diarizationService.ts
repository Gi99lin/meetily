import { invoke } from '@tauri-apps/api/core';
import {
  DiarizationModelsStatus,
  SpeakerDiarizationResult,
  MeetingSpeaker,
} from '@/types/diarization';

export const diarizationService = {
  async getModelsStatus(): Promise<DiarizationModelsStatus> {
    return invoke<DiarizationModelsStatus>('get_diarization_models_status');
  },

  async downloadModels(): Promise<void> {
    return invoke<void>('download_diarization_models');
  },

  async runDiarization(meetingId: string): Promise<SpeakerDiarizationResult> {
    return invoke<SpeakerDiarizationResult>('run_speaker_diarization', { meetingId });
  },

  async isDiarizationRunning(meetingId: string): Promise<boolean> {
    return invoke<boolean>('is_diarization_running', { meetingId });
  },

  async getSpeakerNames(meetingId: string): Promise<MeetingSpeaker[]> {
    return invoke<MeetingSpeaker[]>('get_meeting_speaker_names', { meetingId });
  },

  async renameSpeaker(meetingId: string, speakerKey: string, displayName: string): Promise<void> {
    return invoke<void>('rename_meeting_speaker', { meetingId, speakerKey, displayName });
  },
};
