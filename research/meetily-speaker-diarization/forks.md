# Meetily fork research: speaker diarization

Status: IN PROGRESS

## Method

- The required `using-superpowers` instructions were checked first. Because this is a delegated, bounded research task, its `<SUBAGENT-STOP>` clause says to proceed without applying that skill workflow.

## Repository identity and first-pass inventory

- Searches for the supplied `Gi99lin/meetily` identity did not surface a public repository under that path. The active upstream found by GitHub search is [`Zackriya-Solutions/meetily`](https://github.com/Zackriya-Solutions/meetily), currently described as a privacy-first local meeting assistant and explicitly advertising speaker diarization.
- GitHub's indexed repository page reported roughly 1.3k–2.8k forks depending on crawl snapshot, so a complete manual review of every fork is impractical; the useful inventory is therefore forks/branches with diarization-related code or commits, plus upstream contributions that forks may have absorbed ([upstream repository](https://github.com/Zackriya-Solutions/meetily)).
- One indexed fork, [`Hossamudin/Meetily-forked-hossam`](https://github.com/Hossamudin/Meetily-forked-hossam), only lists speaker diarization as a roadmap item in its README, not as an implemented feature.
- The current upstream `frontend/README.md` itself lists “Speaker diarization support,” indicating that the feature has since landed in the main project and is not solely a private fork experiment ([frontend README](https://github.com/Zackriya-Solutions/meetily/blob/main/frontend/README.md)).
- The upstream README also says its current acceleration strategy is automatic and platform-specific—Metal/CoreML on Apple Silicon and CUDA/Vulkan on Windows/Linux—but that claim applies broadly to model inference and does not by itself establish diarization scheduling or CPU-thread controls ([upstream repository](https://github.com/Zackriya-Solutions/meetily)).

## Related upstream proposals

- Issue [#659](https://github.com/Zackriya-Solutions/meetily/issues/659) proposes MOSS-Transcribe-Diarize as an *optional* experimental backend that performs ASR and diarization in one pass. It explicitly says the present architecture is a separate Whisper/Parakeet + diarization pipeline, and no development branch or pull request is attached. This is an idea, not reusable implementation.
- Issue [#335](https://github.com/Zackriya-Solutions/meetily/issues/335) similarly proposes VibeVoice-ASR for single-pass ASR + diarization. The author recommends asynchronous post-meeting processing because of its 7B-model compute/VRAM cost; again it is a request with no implementation attached.
- The v0.3.0 release notes introduce a post-processing mode for transcription, but do not describe automatic diarization, a shared scheduler, or proportional core allocation ([releases](https://github.com/Zackriya-Solutions/meetily/releases)).

### Interim conclusion

The publicly indexed material supports two architectural directions: keep the existing modular post-processing pipeline, or eventually offer an integrated ASR+diarization model. Neither proposal provides the requested “enabled by default, scheduled automatically, use half the cores” implementation.

## Fork inventory notes

- A direct unauthenticated fetch of GitHub's forks API could not be completed through the web reader (the query URL was rejected as unsafe), so the inventory below relies on GitHub search, repository metadata, and targeted branch/commit inspection rather than pretending to be exhaustive.
- The GitHub CLI is not installed in the workspace environment, so authenticated code/fork search through `gh` was unavailable.
- A sandboxed `curl` request to GitHub's public API failed at DNS resolution; a network-approved retry is required for a machine-readable inventory.
- With network access, the GitHub forks API exposed a high-value candidate: [`TylerBuza/Meetily-ActuallyFree`](https://github.com/TylerBuza/Meetily-ActuallyFree). Its repository description explicitly says it adds diarization along with CPU/Vulkan/CUDA support and removes paywall/account restrictions. At the time of inspection it had been pushed on 2026-08-12, making it both unusually relevant and current ([GitHub forks API](https://api.github.com/repos/Zackriya-Solutions/meetily/forks?sort=stargazers&per_page=100)).
- Other top-starred forks returned by the same inventory include [`qaid/meeting-minutes-autodetect`](https://github.com/qaid/meeting-minutes-autodetect) and [`MaxwellJryao/meeting-minutes`](https://github.com/MaxwellJryao/meeting-minutes). Their descriptions inherit upstream's diarization wording, so description text alone is insufficient evidence of fork-specific work; their diffs must be inspected.

## Relevant fork: Meetily-ActuallyFree

The fork contains a concentrated sequence of fork-specific commits on 2026-08-05, rather than merely inheriting upstream text:

- [`729ec67c1c96`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/729ec67c1c96): “Add on-device speaker diarization…”
- [`f91064408311`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/f91064408311): bundles diarization models with the app.
- [`5580fbdbb8e4`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/5580fbdbb8e4): calibrates diarization against recordings and adds expected-speaker count.
- [`e1e08ffd3dc0`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/e1e08ffd3dc0): explicitly runs speaker diarization automatically when a recording finishes.
- [`6f3a525281f4`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/6f3a525281f4): identifies the local user as a named speaker and adds manual renaming.
- [`e2069dc416d4`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/e2069dc416d4) and [`319ce15f1ceb`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/319ce15f1ceb): persist live speaker labels into the post-call transcript.
- [`ac92dffe8671`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/ac92dffe8671): improves remote-speaker splitting.

This fork has already implemented the central automation requested by the user—post-recording automatic triggering—and likely contains useful UI/data-flow work worth comparing line-by-line.

### Automatic triggering and UI refresh

Commit [`e1e08ffd3dc0`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/e1e08ffd3dc0) implements a simple post-stop orchestration layer in `useRecordingStop.ts`:

- after the meeting is saved, it fire-and-forgets `autoDiarizeMeeting(meetingId)`, so navigation is not blocked;
- the helper first calls `diarization_models_available`; absent models cause a silent no-op rather than an automatic download;
- it waits a fixed 2 seconds for the recording file to flush, then invokes `diarize_meeting` with `numSpeakers: null`;
- success dispatches a browser `meetily:diarization-updated` custom event, and an already-open meeting page refetches transcripts;
- failures are only logged as warnings because live labels remain as fallback.

This solves “no separate click,” but it has no durable job state, retry policy, progress percentage, cancellation, concurrency limit, or resource budget. The fixed 2-second delay is also weaker than a backend completion signal or atomic recording-finalized event.

### Pipeline, models, and progress UI

The initial implementation in [`729ec67c1c96`](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/729ec67c1c96) is fully local Rust/ONNX:

- audio is downmixed/resampled to 16 kHz;
- a pyannote `segmentation-3.0` FP16 ONNX model processes 10-second windows and decodes activity for up to three local speakers per window;
- active speech for each local speaker is converted into Kaldi-compatible 80-bin fbank features, then embedded by a WeSpeaker ResNet34 ONNX model;
- embeddings pass through an x-vector LDA transform from `xvec_transform.npz`, are length-normalized, and clustered globally with in-house average-linkage agglomerative clustering using cosine distance (default stop threshold 0.65 unless the expected speaker count is supplied);
- adjacent same-speaker regions with gaps below 0.5 seconds are merged, then transcript segments receive speaker labels through time alignment.

The manual UI added in the same commit provides only an indeterminate spinner/toast (“Identifying speakers…”) and a final speaker/segment count. It does not emit per-window or per-stage progress. Settings only report whether the three model files exist and where they belong.

No explicit thread-pool sizing, `available_parallelism`, Rayon configuration, process priority, or “half the cores” policy appears in this foundational patch. The algorithm is expressed as serial Rust loops around ONNX calls; actual ONNX intra/inter-op threading therefore depends on session/runtime defaults unless later commits configure it.

## Comparison with the local fork

- The local implementation is structurally different: it uses optional Cargo feature `diarization` backed by `sherpa-onnx`, exposes model download/status/run commands through `frontend/src/services/diarizationService.ts`, and tracks active meeting IDs in a backend `HashSet` mutex to prevent duplicate work ([local Cargo manifest](https://github.com/Zackriya-Solutions/meetily/blob/main/frontend/src-tauri/Cargo.toml) is the nearest upstream reference; these local changes are not yet published at a stable URL).
- Local UI already restores “running” state by querying `is_diarization_running` on mount, supports model download, persists cluster display names, and permits renaming. These are stronger operational pieces than the fork's first implementation.
- The local code search found no automatic post-recording trigger and no diarization inference progress event. It also found no diarization-specific proportional CPU policy; the mere presence of Rayon elsewhere in Cargo does not allocate half the cores to sherpa-onnx.

### Concrete local differences

- Local `DiarizationEngine` delegates the full pyannote segmentation + WeSpeaker embedding + fast clustering pipeline to sherpa-onnx rather than maintaining custom DSP, model wrappers, and agglomerative clustering. This reduces bespoke ML code and follows the same broad model family as `Meetily-ActuallyFree`.
- Local inference is correctly placed inside `tokio::task::spawn_blocking`, so the Tauri async/event loop remains responsive. However, there is one blocking job per invocation and no global semaphore across meetings or model workloads.
- Local transcript attribution now sums overlap duration per speaker before choosing the dominant cluster; this is more robust than selecting a single largest diarized segment.
- Local reruns use a repository-level replacement operation and retain microphone-tagged segments as the known local user. The active-meeting set prevents duplicate diarization for the same meeting, but not simultaneous diarization of different meetings or competition with transcription/summarization.
- The recording-stop hook already waits for transcription completion, flushes buffers, and saves before navigation. This gives a cleaner insertion point for automatic diarization than the external fork's unconditional two-second sleep: trigger after the meeting save has succeeded, ideally via a backend queue/event.

### Current fork file inventory

The current `Meetily-ActuallyFree` tree contains offline modules (`clustering.rs`, `dsp.rs`, `models.rs`), plus later `online.rs`, `voiceprint.rs`, and `download.rs` modules, and bundles all three diarization artifacts under `frontend/src-tauri/resources/diarization` ([repository tree](https://github.com/TylerBuza/Meetily-ActuallyFree/tree/main/frontend/src-tauri/src/diarization), [bundled resources](https://github.com/TylerBuza/Meetily-ActuallyFree/tree/main/frontend/src-tauri/resources/diarization)). This confirms the fork evolved beyond the first offline-only patch into live labeling and voiceprint identification.

The current [`diarization/mod.rs`](https://github.com/TylerBuza/Meetily-ActuallyFree/blob/main/frontend/src-tauri/src/diarization/mod.rs) adds several useful safeguards and accuracy improvements:

- a process-wide async `OPERATION_LOCK` serializes diarization-related operations, preventing concurrent heavy runs and conflicting speaker renames;
- CPU-heavy inference runs through `spawn_blocking`;
- bundled models are preferred only when there is no complete user override;
- clustering is calibrated to threshold 0.60, short/noisy turns cannot seed clusters, and overlap speech is excluded from embeddings but retained as multi-speaker timing;
- dual-track mic/system recordings are handled separately, reserving the local mic as “You”; mixed recordings can use voiceprints/live labels to infer the local cluster;
- transcript writes occur inside a database transaction, named speakers are preserved, and simultaneous speakers can be stored as combined labels.

Even in this evolved version there is still no inference progress callback and no explicit half-core/thread setting in the orchestration module. The process-wide lock chooses predictability over throughput: only one diarization/rename operation runs at a time, but that one job may still consume whatever number of threads ONNX Runtime chooses internally.

Critically, the fork's current [`models.rs`](https://github.com/TylerBuza/Meetily-ActuallyFree/blob/main/frontend/src-tauri/src/diarization/models.rs) constructs both diarization sessions with `CPUExecutionProvider` and graph optimization level 3. It does **not** select CUDA, Vulkan, CoreML, or another accelerator for these models, nor does it call ONNX Runtime intra-op/inter-op thread setters. Therefore the repository-level “CPU/Vulkan/CUDA support” claim should not be read as GPU-accelerated diarization; this particular pipeline is explicitly CPU-backed.

## Search for a sherpa-onnx Meetily fork

Targeted public code searches for the local implementation's distinctive `FastClusteringConfig`, `OfflineSpeakerDiarization`, and `run_speaker_diarization` names did not find another indexed Meetily fork. The local sherpa-onnx approach therefore appears independent/unpublished rather than copied from an discoverable public fork.

## Other inspected fork

- [`qaid/meeting-minutes-autodetect`](https://github.com/qaid/meeting-minutes-autodetect) adds meeting auto-detection and SSR-safe frontend wrappers, but its visible fork-specific commit history contains no diarization implementation or resource-control work. It is not a source for the requested automation.

## Recommendation

Do not replace the local sherpa-onnx implementation wholesale. Reuse the strongest orchestration ideas from [`TylerBuza/Meetily-ActuallyFree`](https://github.com/TylerBuza/Meetily-ActuallyFree): automatic post-save triggering, UI refresh on completion, and a process-wide heavy-operation guard. Keep the local model download integrity checks, duplicate-per-meeting protection, speaker-name persistence, transactional rerun replacement, and summed-overlap assignment.

For the requested default behavior, the safest initial schedule is **after transcription and successful meeting save**, launched in the background. Running diarization concurrently with live transcription would compete for CPU/memory bandwidth and risks degraded real-time captions; the public fork also chose post-recording background refinement rather than concurrent full offline processing ([auto-run commit](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/e1e08ffd3dc0)).

For resources, add an explicit configurable budget rather than assuming “half the cores” happens automatically: compute `max(1, available_parallelism / 2)`, pass it into the inference runtime if sherpa-onnx exposes `num_threads`, and use a global semaphore/lock so transcription, summary inference, and diarization do not each independently saturate the machine. The inspected fork provides serialization but no thread cap, and its diarization sessions are explicitly CPU-only ([current orchestration](https://github.com/TylerBuza/Meetily-ActuallyFree/blob/main/frontend/src-tauri/src/diarization/mod.rs), [model sessions](https://github.com/TylerBuza/Meetily-ActuallyFree/blob/main/frontend/src-tauri/src/diarization/models.rs)).

For progress, emit at least stage-level events (`queued`, `decoding`, `segmenting`, `embedding`, `clustering`, `saving`, `complete`). A real percentage requires either sherpa-onnx callbacks or splitting inference into observable windows; the public fork's spinner and final count are not sufficient for five-minute runs ([manual UI commit](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/729ec67c1c96)).

## Bottom line

Yes, somebody has already built the key “automatic after recording” behavior and a richer live/offline speaker workflow: [`TylerBuza/Meetily-ActuallyFree`](https://github.com/TylerBuza/Meetily-ActuallyFree). But it is not a drop-in answer for proportional hardware use or progress reporting. Its diarization is CPU-only, serialised globally, and still lacks inference progress. The local sherpa-onnx implementation remains the better base; selectively port the scheduling and completion-notification pattern.

Status: COMPLETE
