import type { DiarizationStage } from '@/types/diarization';

export interface DiarizationProgressState {
  label: string;
  value: number | null;
}

export function diarizationProgressForStage(
  stage: DiarizationStage,
): DiarizationProgressState | null {
  switch (stage) {
    case 'queued':
      return { label: 'Queued', value: 8 };
    case 'decoding':
      return { label: 'Preparing audio', value: 25 };
    case 'processing':
      return { label: 'Detecting speakers', value: null };
    case 'saving':
      return { label: 'Saving speaker labels', value: 90 };
    default:
      return null;
  }
}
