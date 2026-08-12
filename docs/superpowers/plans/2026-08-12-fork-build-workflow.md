# Fork Build Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the selected fork commit for macOS Apple Silicon and Windows x64 in GitHub Actions with speaker diarization enabled.

**Architecture:** One manually dispatched workflow uses a two-entry matrix. Matrix values select the runner, Rust target, acceleration features, Tauri bundles, and artifact paths while shared steps install dependencies, build the sidecar and app, verify bundles, and upload artifacts.

**Tech Stack:** GitHub Actions, Tauri 2, Rust stable, Node.js 24, pnpm 11.16.0, `tauri-apps/tauri-action@v0`, `actions/upload-artifact@v4`.

## Global Constraints

- Create `.github/workflows/build-fork.yml`.
- Target only `aarch64-apple-darwin` and `x86_64-pc-windows-msvc`.
- Use matrix `fail-fast: false` and build both targets in parallel.
- Explicitly enable `diarization` in both application builds.
- Produce unsigned Actions artifacts without creating tags or releases.

---

### Task 1: Add the fork build workflow

**Files:**
- Create: `.github/workflows/build-fork.yml`

**Interfaces:**
- Consumes: workspace Cargo packages, `frontend/pnpm-lock.yaml`, Tauri configuration, and the `diarization` feature.
- Produces: `Build Fork Installers`, with macOS and Windows jobs and commit-addressed artifacts.

- [x] **Step 1: Run a failing structural assertion**

```bash
test -f .github/workflows/build-fork.yml \
  && rg -q 'aarch64-apple-darwin' .github/workflows/build-fork.yml \
  && rg -q 'x86_64-pc-windows-msvc' .github/workflows/build-fork.yml \
  && rg -q 'diarization' .github/workflows/build-fork.yml
```

Expected: FAIL because the workflow does not exist.

- [x] **Step 2: Create the workflow**

Create `.github/workflows/build-fork.yml` with this content:

```yaml
name: Build Fork Installers

on:
  workflow_dispatch:

concurrency:
  group: fork-build-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  build:
    name: Build ${{ matrix.name }}
    runs-on: ${{ matrix.runner }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - name: macOS Apple Silicon
            runner: macos-14
            target: aarch64-apple-darwin
            app-features: coreml,diarization
            helper-features: metal
            bundles: dmg,app
            artifact-name: meetily-macos-aarch64-${{ github.sha }}
            artifact-path: |
              target/aarch64-apple-darwin/release/bundle/dmg/*.dmg
              target/aarch64-apple-darwin/release/bundle/macos/*.app
              target/aarch64-apple-darwin/release/bundle/macos/*.app.tar.gz
          - name: Windows x64
            runner: windows-latest
            target: x86_64-pc-windows-msvc
            app-features: vulkan,diarization
            helper-features: vulkan
            bundles: msi,nsis
            artifact-name: meetily-windows-x64-${{ github.sha }}
            artifact-path: |
              target/x86_64-pc-windows-msvc/release/bundle/msi/*.msi
              target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe
    steps:
      - name: Checkout selected commit
        uses: actions/checkout@v4
      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 11.16.0
          run_install: false
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 24
          cache: pnpm
          cache-dependency-path: frontend/pnpm-lock.yaml
      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Cache Rust build
        uses: swatinem/rust-cache@v2
        with:
          workspaces: . -> target
          key: ${{ matrix.target }}-diarization
      - name: Install Vulkan SDK on Windows
        if: runner.os == 'Windows'
        uses: humbletim/install-vulkan-sdk@v1.2
        with:
          version: 1.4.309.0
          cache: true
      - name: Install frontend dependencies
        working-directory: frontend
        run: pnpm install --frozen-lockfile
      - name: Build llama-helper sidecar
        shell: bash
        run: |
          cargo build --release -p llama-helper --features "${{ matrix.helper-features }}"
          extension=""
          if [[ "${{ runner.os }}" == "Windows" ]]; then extension=".exe"; fi
          mkdir -p frontend/src-tauri/binaries
          cp "target/release/llama-helper${extension}" "frontend/src-tauri/binaries/llama-helper-${{ matrix.target }}${extension}"
      - name: Build unsigned Tauri bundles
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          projectPath: frontend
          args: >-
            --target ${{ matrix.target }}
            --bundles ${{ matrix.bundles }}
            --features ${{ matrix.app-features }}
      - name: Verify expected bundles
        shell: bash
        run: |
          if [[ "${{ runner.os }}" == "macOS" ]]; then
            compgen -G 'target/aarch64-apple-darwin/release/bundle/dmg/*.dmg' >/dev/null
          else
            compgen -G 'target/x86_64-pc-windows-msvc/release/bundle/msi/*.msi' >/dev/null
            compgen -G 'target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe' >/dev/null
          fi
      - name: Upload installers
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact-name }}
          path: ${{ matrix.artifact-path }}
          if-no-files-found: error
          retention-days: 14
```

- [x] **Step 3: Re-run the structural assertion**

Expected: PASS with exit code 0.

- [x] **Step 4: Validate YAML and Actions syntax**

```bash
ruby -e 'require "yaml"; YAML.safe_load(File.read(".github/workflows/build-fork.yml"), [], [], true); puts "YAML OK"'
actionlint .github/workflows/build-fork.yml
```

Expected: `YAML OK`; if `actionlint` is installed, it exits 0 without findings.

### Task 2: Verify, commit, push, and dispatch

**Files:**
- Modify: `docs/superpowers/plans/2026-08-12-fork-build-workflow.md`

**Interfaces:**
- Consumes: the reviewed working tree and `origin` fork remote.
- Produces: updated `origin/main` and one Actions run for the exact pushed SHA.

- [x] **Step 1: Run frontend verification**

```bash
cd frontend
bun test
pnpm run build
```

Expected: all Bun tests pass and Next.js exits 0.

- [x] **Step 2: Run Rust verification**

```bash
cargo test -p meetily --features diarization
cargo check -p meetily --no-default-features --features platform-default,coreml,diarization
```

Expected: all non-ignored tests pass and both commands exit 0.

- [x] **Step 3: Review pending content**

```bash
git diff --check
git status --short
git diff --stat
git diff -- .github/workflows/build-fork.yml
```

Expected: no whitespace errors and only the reviewed diarization, UI, documentation, research, tests, and workflow changes are pending.

- [ ] **Step 4: Commit the current version**

```bash
git add .github/workflows/build-fork.yml docs frontend research
git commit -m "feat: automate speaker diarization and fork builds"
```

Expected: one new commit with the implementation and workflow.

- [ ] **Step 5: Push and dispatch**

```bash
git push origin main
gh workflow run build-fork.yml --ref main
gh run list --workflow build-fork.yml --branch main --limit 1
```

Expected: `origin/main` advances without force-push and a queued or in-progress run appears with two matrix jobs. Capture its URL with `gh run view <run-id> --json url,status,conclusion,jobs`.
