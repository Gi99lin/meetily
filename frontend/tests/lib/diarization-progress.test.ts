import { describe, expect, test } from 'bun:test';
import { diarizationProgressForStage } from '../../src/lib/diarization-progress';

describe('diarizationProgressForStage', () => {
  test('uses honest milestones and leaves inference indeterminate', () => {
    expect(diarizationProgressForStage('queued')).toEqual({ label: 'Queued', value: 8 });
    expect(diarizationProgressForStage('decoding')).toEqual({ label: 'Preparing audio', value: 25 });
    expect(diarizationProgressForStage('processing')).toEqual({ label: 'Detecting speakers', value: null });
    expect(diarizationProgressForStage('saving')).toEqual({ label: 'Saving speaker labels', value: 90 });
  });
});
