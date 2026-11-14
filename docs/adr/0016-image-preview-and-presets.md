# ADR 0016: Shared image preview and explicit user presets

Status: proposed, planning only. No product implementation or executable validation.
Date: 2026-11-15.
Spec: docs/specs/2026-11-15-next-features.md.
Implementation plan: docs/superpowers/plans/2026-11-15-next-features.md.

## Context

The customization panel displays a bounded source draft, while the image engine applies orientation, transforms, encoding and optional metadata retention on export. Copying those algorithms into UI would create two inconsistent results. Separately, schema preset support is not a user-preset persistence mechanism, and the declared remember flag is not implemented behavior.

Existing accepted boundaries remain: core validates action options, UI depends on core, engines depend on core, and daemon composes both. The Engine trait serves every format and must not require every adapter to provide an image preview.

## Proposed decisions

1. Add platform-free image preview request/result/provider types in core. Implement the provider in the image engine and inject Arc<dyn ImagePreviewProvider + Send + Sync> through daemon composition into the UI worker.
2. Extract the existing in-memory image preparation/final-byte path so export and preview share it. Preview never submits a job or writes a converted output. The queue remains the only conversion-output finalizer.
3. Distinguish a transformed display proxy from an explicit full encoded preview. Only actually encoded final artifact bytes support an exact byte-count statement. No arbitrary timing or process-RSS guarantee follows from worker threading or admission checks.
4. Use one running and one replaceable pending preview request, cooperative cancellation, and production session/panel/request stamps. A cancelled codec may finish computation but cannot publish into a new panel.
5. Store named user presets in a separate versioned presets.toml file. Use stable IDs, field-ID typed values, a semantic fingerprint and the existing options validator. No file paths, input selections, commands, permissions or secrets in presets.
6. Presets are explicit and scoped to one image action initially. Plain-click defaults, Reset and remember-last-values remain unchanged. No multi-step workflow language or silent migration is introduced.
7. Corrupt/future-version stores are preserved. Schema changes require review or disable the preset. A single daemon writer and revision checks detect ordinary conflicting edits; this is not an arbitrary external-editor transaction guarantee.
8. Existing engine/output/IPC behavior remains compatible unless a later feature packet explicitly documents an additive change and every consumer.

## Alternatives

- UI depends directly on engines: rejected because daemon already provides a composition root and the extra dependency would blur rendering/processing responsibilities.
- Reimplement transforms for a fast UI preview: rejected because orientation, geometry, alpha and metadata behavior would diverge.
- Fully encode every keystroke: rejected because target-byte searches and large codecs can consume substantial CPU and memory.
- Store presets inside Config: rejected for this version because ordinary settings fallback and manual equality create unrelated failure coupling.
- Silently clamp/drop incompatible saved values: rejected because a named setup could then run with a different meaning.
- Adopt nightly filesystem APIs: rejected; keep the current stable toolchain and review replacement semantics at the store boundary.

## Consequences and deferred evidence

Shared preparation is a behavioral refactor and needs later parity evidence, not only a helper unit test. Decoder limits may be best effort and cancellation may be delayed inside a codec. Full-resolution preview admission limits are product policies awaiting measurement. Revalidation and fingerprints add storage complexity but keep action semantics visible.

Native filesystem replacement, worker/resource behavior, actual small-panel layout and later platform adapters need their own acceptance evidence. Tests/builds remain deferred by current owner instruction. This ADR is proposed; its presence does not authorize implementation or establish release readiness.
