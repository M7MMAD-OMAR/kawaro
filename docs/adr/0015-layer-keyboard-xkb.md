# ADR 0015: Translate layer keyboard events with compositor XKB state

Status: accepted for S-139 planning scope by the 2026-11-05 follow-up; implementation and acceptance pending
Date: 2026-11-05

## Context

[S-139](../../backlog/S-139-correct-customization-ui.md) AC3 requires actual typing in numeric Material fields. Lead reports focus and Escape work on the native layer surface, but Ctrl+A, Ctrl+V and direct digits are ignored. The technical audit identifies a navigation-only evdev whitelist, discarded compositor keymap FD and Shift-only modifier handling. Passing generated-control tests is not evidence that the native keyboard path works.

## Decision

Use the compositor XkbV1 keymap and state in a dedicated Linux `layer_keyboard` module behind the existing layer adapter. Authorize only an exact `=0.9.0` direct `xkbcommon` dependency in the UI Linux target section. It is already locked transitively through Slint, declares MIT and uses the existing system libxkbcommon. Do not introduce resolved packages/versions or enable X11 features. The likewise locked `xkbcommon-dl` 0.4.2 is not authorized as an additional direct dependency. Record the binding in [engines](../engines.md).

Consume and close the keymap FD with bounded size and validated format/content; own the keymap/state, use all depressed/latched/locked modifier masks and layout group, then translate XKB keysyms and UTF-8 into Slint press/release events. Preserve Ctrl shortcut identity while suppressing inserted control characters. Cache press text per physical key for matching release events. Clear pressed/modifier state on focus loss, teardown and keymap replacement so stale releases cannot reach new focus/session. Invalid or unsupported maps produce no guessed printable input and cannot panic or leak FDs. Keep the existing pure `overlaykey_for_evdev` nine-key contract unchanged.

## Alternatives and limits

Hardcoded US letters/digits are rejected: they ignore the compositor layout. Expanding the navigation whitelist alone cannot implement text or modifier semantics. A new toolkit/backend or Slint upgrade is unnecessary and outside scope. Compose, IME and additional language input features are deferred; interpreting the supplied non-US XKB layout is required, not a language expansion. Existing window fallback and focus/activation policy stay unchanged.

## Acceptance

Tester-owned real US/non-US keymap fixtures must check exact production events, modifiers/groups, Ctrl editing, cached releases, reset lifecycle and malformed/unreadable/oversized/truncated FD cases. Native layer events must type 25 percent and 321 x 201 and yield the exact decoded outputs required by S-139, with frozen hashes and no setter substitute. Retain window regressions and scoped quality gates. This decision does not close S-139 hint ownership REWORK, S-082 or remaining native evidence.
