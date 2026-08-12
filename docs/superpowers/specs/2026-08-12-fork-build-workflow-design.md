# Fork Build Workflow Design

## Goal

Provide a GitHub Actions workflow that builds the current selected commit of the fork for macOS Apple Silicon and Windows x64 with speaker diarization enabled.

## Scope

- Add one manually dispatched workflow at `.github/workflows/build-fork.yml`.
- Build macOS on `macos-14` for `aarch64-apple-darwin`.
- Build Windows on `windows-latest` for `x86_64-pc-windows-msvc`.
- Build both targets in parallel.
- Explicitly enable the `diarization` feature alongside each platform's acceleration features.
- Produce unsigned test artifacts so the workflow works without signing secrets in the fork.
- Upload macOS DMG/app archives and Windows MSI/NSIS installers as Actions artifacts.

## Workflow Structure

The workflow uses `workflow_dispatch` and a two-entry build matrix. Shared setup installs Node, pnpm, Rust, target toolchains, frontend dependencies, and build prerequisites. Platform-specific steps build `llama-helper`, place its sidecar binary where Tauri expects it, and run `tauri-apps/tauri-action` with explicit feature flags and bundle targets.

The checkout step uses the commit selected when the workflow is dispatched. Artifact names include the platform and commit SHA so builds can be traced back to source.

## Failure Handling

- Matrix fail-fast is disabled so one platform can finish if the other fails.
- Missing expected bundles fail the artifact verification step.
- Artifact upload fails when no matching installer is found.
- No release or tag is created; the workflow only stores downloadable Actions artifacts.

## Verification

- Parse the workflow as YAML locally.
- Run existing frontend tests and production frontend build.
- Run Rust tests and diarization-enabled checks before committing.
- Push the workflow and dispatch it on the pushed `main` commit.
- Confirm both matrix jobs are created and report the Actions run URL/status.
