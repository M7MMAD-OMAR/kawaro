# S-143: Named user presets

Status: store, daemon worker and panel UI wiring source implemented; parent source-reviewed on 2026-11-21. Runtime, visual and restart checks unverified; executable checks deferred by owner order.
Date: 2026-11-15.
Spec: ../docs/specs/2026-11-15-next-features.md
Implementation packet: ../docs/superpowers/plans/2026-11-15-next-features.md

## Story
I want to name a valid image setup and apply it later without changing the default one-click action.

## Current facts
- Core options support field IDs, mapped param keys, clears and validation.
- Action preset machinery exists, but no user-preset store exists; remember is not a production behavior.
- Config fallback and manual equality make independent presets storage preferable.

## Acceptance criteria
- AC1: Store version1, next_id, stable IDs, names, action IDs, fingerprints and allowlisted typed field-ID values in config-dir/presets.toml. Initial Save supports built-in non-PDF image actions and numeric/bool/enum/px-pair controls only; no arbitrary custom action, free text, paths/selections/commands/secrets.
- AC2: Validate through existing OptionsSpec; never silently drop/clamp stale values or reinterpret field IDs as param keys.
- AC3: Keep missing actions or incompatible schemas unavailable, fingerprint changes needing explicit review, future-version files read-only and corrupt files preserved.
- AC4: Use a single daemon writer, revision conflict check and same-directory temp/flush/replacement. Integrate watch/reload without resetting ordinary config.
- AC5: Provide Save current, Apply, Rename and Delete as secondary controls. No automatic last-preset application, job submission or change to Reset.
- AC6: Define name limits, per-action duplicate handling and non-reused checked counter IDs exactly as plan section2.

## Future focused cases, not run
- Save an Arabic-named 25percent setup, restart, apply with zero jobs and one preview update.
- Mutually clearing scale/max-side never become simultaneously active stored values.
- Removed action, changed type/range, corrupt store and unknown future version preserve data.
- External edit before Save yields conflict; deleted preset IDs are not reused.

## Boundaries
No workflow chains, preset import/export, CLI/MCP management, remember-last implementation or new config coupling.

## Delivery and review
Parent owns architecture, packet approval and final review. Selected Pi/MiniMax-M3.1-Flash executes only a later authorized bounded packet. Native platform qualification follows stable feature behavior. Source implementation, executed acceptance and public release are recorded separately.

## Planning log
- 2026-11-15: parent authored this story from current source research and the owner request. No product code or executable check changed or ran.
