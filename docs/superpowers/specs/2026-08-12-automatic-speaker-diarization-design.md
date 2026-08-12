# Automatic speaker diarization design

## Goal

Make offline speaker diarization the default post-transcription workflow, keep
the application responsive, and expose useful progress without replacing the
existing `sherpa-onnx` implementation.

## Decisions

- Speaker diarization is enabled by default for new installations and remains
  user-disableable through the existing beta-feature setting.
- A newly saved meeting is enqueued only after its final transcript and audio
  metadata have been persisted successfully.
- Offline diarization does not run concurrently with live transcription. A
  single global permit serializes diarization jobs; the existing per-meeting
  guard continues to reject duplicate requests.
- Segmentation and speaker-embedding sessions each receive
  `max(1, available_parallelism / 2)` threads. They execute sequentially, so
  they share rather than double the CPU budget.
- The backend emits stage events (`queued`, `decoding`, `segmenting`, `saving`,
  `complete`, `failed`). No fabricated percentage is shown for model inference.
- Completion causes an open meeting view to refetch transcripts and speaker
  names. Failure leaves the saved transcript untouched and permits a manual
  retry.

## Data flow

`recording stopped -> transcription finalized -> meeting saved -> enqueue ->
global permit -> decode -> diarize -> transactional label replacement -> event
-> UI refresh`

The frontend auto-enqueue is fire-and-forget: navigation and the successful
save notification are not delayed by model work. The backend owns job state so
navigating away cannot cancel or duplicate the operation. If models are absent,
automatic processing reports a non-fatal failure and the existing manual flow
can download them and retry.

## Components

1. `AppState` owns the global inference permit and active meeting IDs.
2. Diarization commands share one internal job function used by both manual and
   automatic entry points.
3. `DiarizationEngine` accepts an explicit thread count and applies it to the
   segmentation and embedding configurations.
4. The recording-stop hook invokes auto-enqueue after a successful save when
   the persisted feature flag is enabled.
5. Meeting transcript controls listen for status events and refetch on
   completion while preserving the manual rerun action.

## Error handling

- Duplicate jobs for one meeting return the existing clear error.
- Jobs for different meetings wait for the single global permit.
- Model, audio, inference, or database errors emit `failed`, remove active state,
  and do not make meeting saving fail.
- Transcript labels are mutated only by the existing transactional replacement
  after successful inference.

## Tests and verification

- Unit-test the relative thread calculation, including one- and two-core hosts.
- Unit-test job-state transitions and serialization at the smallest practical
  boundary.
- Test that the frontend auto-enqueue predicate defaults on and respects an
  explicit disabled setting.
- Run targeted frontend/Rust tests, both Cargo feature configurations, the
  frontend production build, and the desktop development launch.

## Non-goals

- No live speaker clustering during recording.
- No custom DSP/ONNX implementation copied from another fork.
- No GPU-provider changes, biometric identity, cancellation, retry scheduler,
  or fabricated numeric inference progress in this change.
