# Lightweight development, research and build workflow

Date: 2026-11-15. Source/process audit, not benchmark evidence.
Current owner mode: source implementation and review only. Do not run the future build/check commands described here while execution is deferred.

## Current facts

- This repository has no installed factory/ directory or mission command. Its current execution uses bounded Pi/MiniMax packets with parent plans and source review.
- Native Cargo artifacts belong to the existing project target; Windows cross artifacts use the existing separate kawaro-win-target cache. Never copy target into a scratch checkout or clear caches to repair a test.
- Development already uses line-table debug information and dependency debug=0. Release uses thin LTO and codegen-units=4. Profile changes create different artifacts and need their own later validation.
- Settings and other visual scripts force a release example build and use release/examples paths. Routine visual work therefore cannot choose a lighter profile today.
- Windows scope currently requests CPUQuota=200%, and multiple Windows entry points use -j2. The latest local policy is CPUQuota=100%, MemoryMax=6G, nice15, one Cargo job, Rust test threads2 and temp files under HOME. This mismatch is a recorded repair target, not an approved extra allocation.
- The historical release-gates.sh archive/mutation procedure is already disallowed by the owner. Do not invoke it or present it as the current release gate.

## Four explicit modes

| Mode | Purpose | Future scope | Current execution |
| --- | --- | --- | --- |
| source | Edit/review code, contracts and test source | Exact changed files; no executable validation | Allowed |
| check | Discriminating affected-feature verification | Smallest meaningful existing cases and affected consumers | Deferred |
| visual | Private-compositor UI acceptance | Explicit example/profile/target and matched capture conditions | Deferred |
| release | Frozen candidate acceptance and distribution | Complete required native/cross/package set once, then rerun only after relevant changes/failures | Deferred |

A future entry point should print the planned commands, profile, target/cache, affected cases and resource policy by default. Actual execution requires an explicit run mode after the owner's execution deferral is lifted. No script named dry-run should unexpectedly build, install or run a compositor. This document adds no such executable wrapper yet.

## Repair order

1. Consolidate local resource ownership in one existing-style wrapper. Apply 6G/CPU100/nice15/jobs1/testthreads2 and HOME temp storage at every heavy local entry. Update scripts and meaningful resource-contract expectations together; never hide a mismatch with allow flags or delete assertions. Keep CI behavior explicit rather than spoofing CI to avoid a scope.
2. Introduce explicit visual profile/binary selection without changing release defaults. A dev/visual capture is labeled as that artifact, not release evidence. No claim that its pixels match release until actual later comparison.
3. Define a source-to-case map from current tests, including shared-core dependency consumers, platform siblings and test-fixture ownership. No selected case for a source change is a mapping gap, not a pass. Docs-only work does not trigger an unrelated full Rust rebuild by default.
4. Reuse stable target/toolchain/flags per mode. Do not toggle RUSTFLAGS, target triples or profile settings between ordinary packets and expect one cache to remain valid. Inspect Cargo fingerprints before diagnosing a warm build as cold.
5. Keep full release checks serialized on frozen source. Mutation remains optional, bounded and outside the release gate. No parallel target copies and no -j4 historical archive loop.
6. Evaluate new runners/caches only when measurements are allowed. Keep cargo test as the current default until a real representative comparison justifies migration.

## Research and agent packets

- Batch independent primary-source queries, read the first supporting result, and keep its date and URL. Do not open a browser to reconfirm the same snippet.
- Use exact graph symbols/path spans for source context. Historical research does not override current source.
- Classify every outcome as missing, source implemented, runtime unverified, or publicly released. These are different states.
- Each executor receives allowed paths, reuse map, explicit contracts, stop conditions and forbidden commands. Disjoint packets may run in parallel; shared-file integration is serialized.
- Parent reviews the actual current diff, not an agent's confident summary. Fix a small finding directly and record authorship; a new independent review uses a new task/context.
- Updates lead with the user's visible result and a real stage change. No repeated permission question when current authorization covers the action, no long sleep/poll loop, and no invented speed/cost savings.

## Factory and skill integration later

The canonical factory is a separate project. Do not run its mission/watchdog/install commands against this repository, mutate another queue, or change shared/global skills as a shortcut.

Later integration must reconcile: owner model routing, actual host enforcement, backlog-to-ticket identity, local resource policy, test execution mode, protected paths and factory-owned short-lived worktrees. Its automatic gates currently conflict with the source-only mode. A cached skill's Claude hook/frontmatter is not proof of enforcement under Codex/Pi. Preserve real model attribution.

The next process changes belong in this project first. A reusable skill update is considered only after the project workflow is correct and a concrete global scope is established; no unmeasured optimization is installed machine-wide.

## User-facing gaps found

Current IPC provides CancelAll but not a single-job cancel or job-list query. Progress visibility depends on the tray; absence of an icon does not mean conversion execution stops. The S-144 ledger provides the foundation, but a non-tray entry and per-job control need an explicit follow-up story, S-148. Keep cancellation instance-aware so a stale ID from a previous daemon cannot cancel an unrelated new job.

## Primary references

[Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) explains custom profile inheritance and separate artifacts. [Cargo build cache](https://doc.rust-lang.org/stable/cargo/reference/build-cache.html) explains target/profile directories and flags affecting builds. These support stable-mode design, not a measured speedup.

[sccache Rust limitations](https://github.com/mozilla/sccache/blob/main/docs/Rust.md) identifies incremental-compilation and linker restrictions, so sccache is not an automatic remedy for this release/link-heavy workflow. [nextest running guide](https://nexte.st/docs/running/) describes the alternative runner; adopting it needs preserved test semantics and an actual resource-limited comparison later.

No build/test/lint/benchmark/installer/factory/CI run was performed for this audit.
