# Speaker Diarization Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make speaker diarization start reliably, assign speakers by cumulative overlap, reject partial model downloads, and atomically replace stale rerun results.

**Architecture:** Keep the current Tauri/frontend flow. Harden three focused Rust boundaries: pure overlap attribution in `engine.rs`, model finalization in `models.rs`, and transactional persistence in `transcript.rs`; retain the existing system-font frontend patch to remove the startup network dependency.

**Tech Stack:** Rust 2024, Tokio, SQLx/SQLite, sherpa-onnx, Tauri 2, Next.js 14, React 18, Bun.

## Global Constraints

- Preserve all existing high-confidence `mic` labels.
- Do not add dependencies or redesign the Tauri command surface.
- Do not modify unrelated warnings or the pre-existing FFmpeg-dependent test.
- Preserve the user's current `globals.css` and `layout.tsx` changes.
- Work in the current checkout because those uncommitted startup changes are part of the requested fix; do not rewrite or discard them.

---

### Task 1: Attribute by cumulative speaker overlap

**Files:**
- Modify/Test: `frontend/src-tauri/src/diarization/engine.rs`

**Interfaces:**
- Consumes: `DiarizedSegment { start_seconds, end_seconds, speaker_index }`.
- Produces: unchanged `dominant_speaker_key_for_range(&[DiarizedSegment], f64, f64) -> Option<String>`.

- [ ] **Step 1: Add a failing cumulative-overlap regression test**

```rust
#[test]
fn test_dominant_speaker_key_aggregates_overlap_per_speaker() {
    let diarized = vec![
        segment(0.0, 2.0, 0),
        segment(2.0, 5.0, 1),
        segment(5.0, 7.0, 0),
    ];
    assert_eq!(
        dominant_speaker_key_for_range(&diarized, 0.0, 7.0),
        Some("speaker_00".to_string())
    );
}
```

- [ ] **Step 2: Run the isolated test and verify RED**

Run: `cargo test --lib --features diarization diarization::engine::tests::test_dominant_speaker_key_aggregates_overlap_per_speaker -- --exact`

Expected: FAIL because the current implementation selects speaker 1's single 3-second segment instead of speaker 0's cumulative 4 seconds.

- [ ] **Step 3: Aggregate overlap by speaker index**

Use a `BTreeMap<i32, f64>` so every positive overlap is added to that speaker's total, then select the largest total with a deterministic speaker-index tie break and format it through `speaker_key`.

- [ ] **Step 4: Run all diarization-engine tests and verify GREEN**

Run: `cargo test --lib --features diarization diarization::engine::tests`

Expected: all engine tests pass.

### Task 2: Make model downloads atomic and readiness strict

**Files:**
- Modify/Test: `frontend/src-tauri/src/diarization/models.rs`

**Interfaces:**
- Consumes: the existing fixed asset URLs and `download_file(url, dest, progress)` call sites.
- Produces: final model files only after a complete streamed response; existing `ensure_models_downloaded` API remains unchanged.

- [ ] **Step 1: Add a failing readiness regression test**

Create a temporary models directory, write an embedding model file of `EMBEDDING_MODEL_SIZE_BYTES - 1` bytes, and assert `check_models_status(...).embedding_ready` is false. Then resize it to exactly `EMBEDDING_MODEL_SIZE_BYTES` and assert readiness is true.

- [ ] **Step 2: Run the isolated test and verify RED**

Run: `cargo test --lib --features diarization diarization::models::tests::embedding_model_requires_complete_expected_size -- --exact`

Expected: FAIL on the truncated file because the current half-size threshold reports it ready.

- [ ] **Step 3: Tighten readiness and atomic finalization**

Require the embedding final file to equal `EMBEDDING_MODEL_SIZE_BYTES`. In `download_file`, stream into a sibling UUID-suffixed `.part` path, verify that the downloaded count equals the HTTP `Content-Length` when present, call `sync_all`, then rename the part file to `dest`. Remove the part file on every error. Keep the final path untouched until validation succeeds.

- [ ] **Step 4: Run model and full diarization tests**

Run: `cargo test --lib --features diarization diarization::models::tests`

Expected: all model tests pass and no network is used.

### Task 3: Replace rerun results transactionally

**Files:**
- Modify/Test: `frontend/src-tauri/src/database/repositories/transcript.rs`
- Modify: `frontend/src-tauri/src/diarization/commands.rs`

**Interfaces:**
- Produces: `TranscriptsRepository::replace_diarization_results(pool: &SqlitePool, meeting_id: &str, assignments: &[(String, String)]) -> Result<(), SqlxError>`.
- Consumes: transcript ID/new speaker-key pairs computed after successful inference.

- [ ] **Step 1: Add a failing in-memory SQLite regression test**

Create minimal `meetings`, `transcripts`, and `meeting_speakers` tables. Insert one `mic` row, one stale diarized row that receives a new assignment, one stale diarized row with no new overlap, and one saved speaker name. Call `replace_diarization_results`; assert the mic label remains, the matched row receives its new key, the unmatched row becomes `NULL`, and meeting speaker names are empty.

- [ ] **Step 2: Run the isolated repository test and verify RED**

Run: `cargo test --lib --features diarization database::repositories::transcript::tests::replace_diarization_results_is_atomic_and_clears_stale_state -- --exact`

Expected: compilation fails because the repository method does not exist.

- [ ] **Step 3: Implement the transaction**

Begin a SQLx transaction; set `speaker = NULL` for every meeting transcript whose speaker is not `mic`; apply each `(transcript_id, speaker_key)` update scoped to the meeting and excluding `mic`; delete `meeting_speakers` rows for the meeting; commit.

- [ ] **Step 4: Switch the command to one atomic write**

Build the assignment vector in memory after inference. Count its length for `segments_updated`, call `replace_diarization_results` once, and remove the per-row `update_speaker` loop. Inference errors must occur before this call and therefore leave the database unchanged.

- [ ] **Step 5: Run repository and diarization tests**

Run: `cargo test --lib --features diarization database::repositories::transcript::tests diarization::engine::tests diarization::models::tests`

Expected: all targeted tests pass.

### Task 4: Verify startup and the complete change set

**Files:**
- Verify: `frontend/src/app/layout.tsx`
- Verify: `frontend/src/app/globals.css`
- Verify all changed Rust files from Tasks 1-3.

**Interfaces:**
- Consumes: completed fixes from Tasks 1-3.
- Produces: evidence that frontend startup assets, both Cargo feature configurations, and tests remain healthy.

- [ ] **Step 1: Run formatting and diff checks**

Run: `cargo fmt --all -- --check` and `git diff --check`.

Expected: both exit 0.

- [ ] **Step 2: Run frontend checks**

Run: `bun test` and `pnpm run build` from `frontend/`.

Expected: 8 frontend tests pass and Next.js production build exits 0 without a Google-font fetch.

- [ ] **Step 3: Run Rust compilation in both feature modes**

Run: `cargo check --all-targets` and `cargo check --all-targets --features diarization`.

Expected: both exit 0.

- [ ] **Step 4: Run the Rust library suite**

Run: `cargo test --lib --features diarization`.

Expected: all diarization-related tests pass. If only `audio::incremental_saver::tests::test_checkpoint_creation` fails because runtime FFmpeg discovery cannot see the bundled binary, record it as the known baseline failure.

- [ ] **Step 5: Launch the desktop application**

Run: `pnpm run tauri:dev` from `frontend/`, inspect the terminal for startup errors, and confirm that the main WebView renders.

Expected: the Meetily window displays its UI rather than a blank white page.

- [ ] **Step 6: Review the final diff**

Confirm every changed line maps to the approved design, no user change was lost, and no generated build artifacts are tracked. Do not commit unless the user explicitly requests it.
