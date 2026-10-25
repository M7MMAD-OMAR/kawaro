[English](CONTRIBUTING.md) | [العربية](CONTRIBUTING.ar.md)

# Contributing to Kawaro

Thanks for your interest. Read `docs/architecture.md` first and open an issue
to discuss any large change before writing it.

## Before you open a pull request

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/design-lint.sh crates/ui
```

`scripts/release-gates.sh` runs everything a release needs, including the
end-to-end and focus tests in private headless compositors (KWin and GNOME
Shell). It never touches your running desktop.

## Rules

- No file leaves the user's machine: no servers, no telemetry.
- A new format is one TOML file in `actions/` plus a small adapter in
  `crates/engines/`. No action logic in the UI.
- Never modify the original file. Engines write a temporary file; the queue
  renames it atomically.
- Every user-facing string in Arabic and English, with right to left layout.
- Design rules are in `docs/design-system.md`: no shadows, no gradients, no
  hand-written colours or sizes; everything comes from `design/tokens.json`.
- No long dashes (em dash or en dash) in code, comments or documentation.
- Check the licence of any new dependency and record it in `docs/engines.md`.
- Commit messages in English. One line per change in `CHANGELOG.md` under
  `[Unreleased]`.

By contributing you agree that your work is licensed under GPL-3.0-or-later.
