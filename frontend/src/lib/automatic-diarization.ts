export async function enqueueAutomaticDiarization(
  enabled: boolean,
  meetingId: string,
  enqueue: (meetingId: string) => Promise<void>,
): Promise<void> {
  if (!enabled) return;
  await enqueue(meetingId);
}
