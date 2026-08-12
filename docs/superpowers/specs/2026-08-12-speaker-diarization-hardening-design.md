# Speaker diarization hardening design

## Goal

Make the opt-in offline speaker-diarization path start reliably and produce internally consistent speaker labels without broad architectural changes.

## Scope

The change keeps the existing Tauri command, database schema, model choices, and frontend workflow. It addresses four observed failure modes:

1. The initial Next.js development compile must not depend on downloading a Google font. The existing local change replacing `next/font/google` with a system font stack remains part of the fix.
2. Transcript attribution must select the speaker with the greatest cumulative overlap across all diarization segments, not the speaker owning the single longest overlapping segment.
3. Interrupted model downloads must never leave a partial file that is reported as ready. Downloads use a sibling temporary file, validate the completed byte count when an expected size is known, flush the file, and atomically rename it into place only after validation.
4. A repeated diarization run must not mix fresh clustering results with stale labels or names. After inference succeeds, all non-`mic` transcript labels are replaced in one database transaction: matched rows receive their new cluster key and unmatched rows become `NULL`. Existing custom names for the meeting are cleared in the same transaction because cluster indices are not stable across runs.

## Data flow

The frontend checks model status, downloads missing assets, then invokes `run_speaker_diarization`. Rust decodes the recording and performs inference off the async runtime. The pure attribution function aggregates positive overlap duration by speaker key. Only after inference and attribution complete does the repository apply the full label set transactionally and clear stale speaker-name mappings. The command then returns counts and the frontend refetches transcripts and speaker names.

Failed inference or attribution performs no database mutation. Failed database replacement rolls back all label and name changes. A failed or interrupted download leaves only a `.part` file; the final model path remains absent and the next attempt retries. Existing high-confidence `mic` labels remain unchanged.

## Tests

- Add a pure regression test where two shorter segments from one speaker cumulatively exceed a longer segment from another speaker.
- Add model-file tests showing incomplete files are not ready and validating the temporary-file/final-file boundary without network access.
- Add repository tests if the existing database test harness can exercise an atomic replacement cheaply; otherwise isolate and test the replacement query inputs while verifying the transaction code through compilation.
- Run `bun test`, `pnpm run build`, `cargo check --all-targets` with and without `diarization`, and `cargo test --lib --features diarization`. The unrelated FFmpeg-dependent checkpoint test may require the bundled FFmpeg path and must be reported separately if it remains the only failure.
- Launch `pnpm run tauri:dev` and confirm that the main window renders rather than remaining white.

## Non-goals

This work does not add cancellation, multi-window download coordination, persistent background jobs, speaker identity matching across diarization runs, or new UI. Those are separate enhancements rather than prerequisites for a correct minimal fix.
