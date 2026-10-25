[English](contribute.md) | [العربية](contribute.ar.md)

# Contribute

- Add a format: one TOML file in `actions/` plus a small adapter in `crates/engines/`. No action logic in the UI.
- Never modify the original file. Engines write a temporary file, and the queue renames it.
- Every user string in Arabic and English, with right to left layout.
- Colours, sizes and durations come from `design/tokens.json`.
- Read [CONTRIBUTING](https://github.com/M7MMAD-OMAR/kawaro/blob/main/CONTRIBUTING.md) and open an issue before a large change.

The code is GPL-3.0-or-later: [github.com/M7MMAD-OMAR/kawaro](https://github.com/M7MMAD-OMAR/kawaro).
