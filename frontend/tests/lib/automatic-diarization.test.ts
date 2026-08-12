import { describe, expect, mock, test } from 'bun:test';
import { enqueueAutomaticDiarization } from '../../src/lib/automatic-diarization';

describe('enqueueAutomaticDiarization', () => {
  test('queues a completed import when speaker diarization is enabled', async () => {
    const enqueue = mock(async (_meetingId: string) => {});

    await enqueueAutomaticDiarization(true, 'meeting-imported', enqueue);

    expect(enqueue).toHaveBeenCalledWith('meeting-imported');
  });

  test('does not queue when the saved preference is disabled', async () => {
    const enqueue = mock(async (_meetingId: string) => {});

    await enqueueAutomaticDiarization(false, 'meeting-imported', enqueue);

    expect(enqueue).not.toHaveBeenCalled();
  });
});
