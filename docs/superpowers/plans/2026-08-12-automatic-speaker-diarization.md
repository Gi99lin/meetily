# Automatic Speaker Diarization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Automatically run speaker diarization after a saved transcription, expose honest job stages, and cap its CPU use at half the logical cores.

**Architecture:** Keep the current `sherpa-onnx` inference and transactional persistence. Add a non-blocking Tauri enqueue command backed by the existing active-meeting set plus one global semaphore, emit lifecycle events from the shared job path, and trigger it from the frontend only after `saveMeeting` returns a meeting ID.

**Tech Stack:** Rust 2024, Tokio, Tauri 2, sherpa-onnx 1.13.4, TypeScript, React 18, Bun.

## Global Constraints

- Preserve existing `mic` labels and transactional replacement semantics.
- Never delay or fail meeting saving/navigation because diarization fails.
- Do not run offline diarization before live transcription and recording save finish.
- Use `max(1, logical_cores / 2)` threads for both sequential sherpa-onnx stages.
- Do not add dependencies, GPU providers, live clustering, cancellation, or fake percentages.

---

### Task 1: Default-on frontend policy and post-save enqueue

**Files:**
- Modify/Test: `frontend/src/types/betaFeatures.ts`
- Create: `frontend/tests/types/betaFeatures.test.ts`
- Modify: `frontend/src/services/diarizationService.ts`
- Modify: `frontend/src/hooks/useRecordingStop.ts`

**Interfaces:**
- Produces: `DEFAULT_BETA_FEATURES.speakerDiarization === true`.
- Produces: `diarizationService.enqueueDiarization(meetingId: string): Promise<void>`.
- Consumes: `betaFeatures.speakerDiarization` from `useConfig()`.

- [ ] **Step 1: Add the failing default-policy test**

```ts
import { describe, expect, test } from 'bun:test';
import { DEFAULT_BETA_FEATURES } from '../../src/types/betaFeatures';

describe('speaker diarization defaults', () => {
  test('is enabled for installations without a saved override', () => {
    expect(DEFAULT_BETA_FEATURES.speakerDiarization).toBe(true);
  });
});
```

- [ ] **Step 2: Run the test and verify RED**

Run: `bun test tests/types/betaFeatures.test.ts` from `frontend/`.
Expected: FAIL because the current default is `false`.

- [ ] **Step 3: Make the default true and add enqueue API**

Set `speakerDiarization: true` while leaving persisted overrides untouched. Add:

```ts
enqueueDiarization(meetingId: string): Promise<void> {
  return invoke<void>('enqueue_speaker_diarization', { meetingId });
}
```

- [ ] **Step 4: Trigger enqueue after successful meeting save**

Read `betaFeatures` through `useConfig()`. Immediately after the saved `meetingId` is validated, start but do not await:

```ts
if (betaFeatures.speakerDiarization) {
  void diarizationService.enqueueDiarization(meetingId).catch((error) => {
    console.warn('Automatic speaker diarization could not be queued:', error);
  });
}
```

Add `betaFeatures.speakerDiarization` to the hook callback dependencies.

- [ ] **Step 5: Run the policy test and TypeScript build**

Run: `bun test tests/types/betaFeatures.test.ts && pnpm run build` from `frontend/`.
Expected: one test passes and the Next.js build exits 0.

### Task 2: Relative sherpa-onnx CPU budget

**Files:**
- Modify/Test: `frontend/src-tauri/src/diarization/engine.rs`

**Interfaces:**
- Produces: `inference_threads_for(logical_cores: usize) -> i32`.
- `DiarizationEngine::new` applies that value to segmentation and embedding `num_threads`.

- [ ] **Step 1: Add failing thread-budget tests**

```rust
#[test]
fn inference_threads_use_half_the_logical_cores() {
    assert_eq!(inference_threads_for(1), 1);
    assert_eq!(inference_threads_for(2), 1);
    assert_eq!(inference_threads_for(8), 4);
    assert_eq!(inference_threads_for(10), 5);
}
```

- [ ] **Step 2: Run the isolated test and verify RED**

Run: `cargo test --lib --features diarization diarization::engine::tests::inference_threads_use_half_the_logical_cores -- --exact` from `frontend/src-tauri/`.
Expected: compilation fails because `inference_threads_for` does not exist.

- [ ] **Step 3: Implement and apply the budget**

Implement a pure helper using `logical_cores.max(1) / 2`, clamped to at least one and `i32::MAX`. Resolve the host value using `std::thread::available_parallelism()`, then set the same `num_threads` on `OfflineSpeakerSegmentationModelConfig` and `SpeakerEmbeddingExtractorConfig`.

- [ ] **Step 4: Run all engine tests**

Run: `cargo test --lib --features diarization diarization::engine::tests`.
Expected: all engine tests pass.

### Task 3: Global queue and lifecycle events

**Files:**
- Modify: `frontend/src-tauri/src/state.rs`
- Modify: `frontend/src-tauri/src/database/setup.rs`
- Modify: `frontend/src-tauri/src/database/commands.rs`
- Modify/Test: `frontend/src-tauri/src/diarization/commands.rs`
- Modify: `frontend/src-tauri/src/lib.rs`

**Interfaces:**
- `AppState` produces `diarization_permit: Arc<tokio::sync::Semaphore>` initialized with one permit.
- Produces Tauri command `enqueue_speaker_diarization(app, meeting_id) -> Result<(), String>`.
- Produces event `diarization-status` with `{ meeting_id, stage, error? }`.
- Manual and automatic requests consume the same internal runner and guards.

- [ ] **Step 1: Add a failing serialization test**

Add a Tokio test around a small `single_job_semaphore()` constructor: acquire its first permit, spawn a second acquisition, assert the second has not completed, release the first, then assert the second completes.

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test --lib --features diarization diarization::commands::tests::diarization_jobs_are_serialized -- --exact`.
Expected: compilation fails because the semaphore constructor does not exist.

- [ ] **Step 3: Add shared state and initialization**

Add the one-permit semaphore to every `AppState` initializer in setup/test paths. Keep the existing `diarizing_meetings` set for same-meeting duplicate detection.

- [ ] **Step 4: Refactor the command into one guarded runner**

Create an internal async runner that:

1. inserts the meeting ID into the active set;
2. emits `queued`;
3. awaits the global permit;
4. calls the existing inner inference function;
5. emits `complete` or `failed`;
6. removes the active ID on every outcome.

Emit `decoding`, `processing`, and `saving` immediately before the corresponding existing boundaries. The manual command awaits this runner. The enqueue command spawns it with `tauri::async_runtime::spawn` and returns once scheduled.

- [ ] **Step 5: Register the enqueue command and verify GREEN**

Run: `cargo test --lib --features diarization diarization::commands::tests::diarization_jobs_are_serialized -- --exact`.
Expected: the serialization test passes.

### Task 4: Event-driven meeting refresh

**Files:**
- Modify: `frontend/src/types/diarization.ts`
- Modify: `frontend/src/components/MeetingDetails/TranscriptButtonGroup.tsx`

**Interfaces:**
- Consumes `diarization-status` payload `{ meeting_id, stage, error?: string }`.
- Completion calls the existing transcript and speaker-name refetch callbacks.

- [ ] **Step 1: Define the status payload type**

Add a string-literal stage union for `queued | decoding | processing | saving | complete | failed` and a payload interface with `meeting_id`, `stage`, and optional `error`.

- [ ] **Step 2: Listen for backend state**

Register one Tauri event listener scoped to `meetingId`. Set the spinner for active stages, clear it for terminal stages, and on `complete` set `hasResults` plus await both existing refetch callbacks. Preserve polling as a fallback for remount/reconnect.

- [ ] **Step 3: Verify the frontend build**

Run: `pnpm run build` from `frontend/`.
Expected: Next.js compiles without TypeScript errors.

### Task 5: Full verification and desktop launch

**Files:**
- Verify all files modified by Tasks 1-4.

**Interfaces:**
- Produces fresh evidence for tests, builds, both feature modes, and runtime startup.

- [ ] **Step 1: Check formatting and patch integrity**

Run: `cargo fmt --all -- --check` from `frontend/src-tauri/`, then `git diff --check` from the repository root.
Expected: both exit 0.

- [ ] **Step 2: Run frontend tests and production build**

Run: `bun test && pnpm run build` from `frontend/`.
Expected: all tests pass and the build exits 0.

- [ ] **Step 3: Compile Rust with and without diarization**

Run: `cargo check --all-targets` and `cargo check --all-targets --features diarization` from `frontend/src-tauri/`.
Expected: both exit 0.

- [ ] **Step 4: Run Rust library tests**

Run: `cargo test --lib --features diarization` from `frontend/src-tauri/`.
Expected: all diarization tests pass; report any pre-existing FFmpeg-environment failure separately.

- [ ] **Step 5: Launch the application**

Run: `pnpm run tauri:dev` from `frontend/` and inspect the terminal for frontend, Rust, and WebView startup errors.
Expected: the desktop application reaches its normal UI and remains responsive.

- [ ] **Step 6: Review the final diff**

Confirm that no user changes were overwritten, no generated artifacts are tracked, and every changed line maps to the approved design.
