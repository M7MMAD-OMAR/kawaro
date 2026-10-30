# S-108 Hive cells for unsupported files (S-096b)
Status: draft

## Story
As a user who selects files Kawaro cannot fully convert, I want greyed cells and a request cell in the hive, so that I see which files are skipped and can ask for the format with one click.

## Scope (from S-096, the part that needs ui seams or a compositor)
- AC2 centre message as its own line below the centre text (today the daemon joins it with a newline).
- AC3 spies: 0 child processes, 0 network calls for a missing engine hive.
- AC4 one greyed cell per unsupported file (extension code only, up to 6, then `+K`), not activatable, full reason in the centre on hover; a real action runs on the supported files only.
- AC5 `REQ` cell, label and hover text from `kawaro_core::unsupported`, opener through the existing opener module only (0 launches on show, 1 on click).
- AC7 suggestion line in the centre.
- AC8 tokens only, accessible labels in both languages, reduced motion, 12 screenshots.
- AC9 hive within 150 ms, 3 trials in the private compositor.

## Acceptance criteria
As the listed S-096 criteria; no new behaviour.

## Review log
- 2026-10-30, implementer: drafted from the S-096 split.
