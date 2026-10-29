# S-094b About page actions: links, copy version, update shortcut
Status: draft (2026-10-29)

## Story
As a user, I want the About page to open the website and source code, copy my version, and jump to update checking, so that I can report a bug or update without leaving the app.

## Dependency
After S-094 (the About page and shell exist).

## Acceptance criteria
- AC1: the About page shows two links, website `https://kawaro.sbarah.com/` and source `https://github.com/M7MMAD-OMAR/kawaro`, each with an accessible label naming its target.
- AC2: one function `open_url(&str)` in `kawaro_ui` opens a link in the default browser on Linux and Windows. It accepts only those two exact URLs (anything else returns an error and spawns nothing). It starts the browser without raising or focusing any Kawaro window and opens no Kawaro window. On Linux it reuses the command builder pattern of `notify_logic.rs` (the S-049 source test forbids spelling `xdg-open` elsewhere in the daemon); on Windows it uses `ShellExecuteW` with the `open` verb.
- AC3: a "Copy version" button puts exactly the version string (0.4.0 today) on the system clipboard and shows "Copied" for 2000 ms, then returns to the normal label.
- AC4: a "Check for updates" button on About selects the Updates page and sends exactly one `Request::UpdateCheck`, the same callback as the Updates page button; it is disabled while a check runs.
- AC5: Arabic and English strings, RTL mirrored, labelled controls, tokens only.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Find both links by label; their text equals the two URLs | ui |
| AC2 | Command builder returns `("xdg-open", [url])` for each allowed URL and an error for `file:///etc/passwd`, `http://x` and an empty string; the spy launcher is called 0 times on error; window count unchanged | unit, ui |
| AC3 | Press the button with a fake clipboard: content equals the version string; label is "Copied" before 2000 ms and normal after | ui |
| AC4 | Press with a fake IPC: current page index 3, one `update-check` request, button disabled until the reply | integration |
| AC5 | Strings non empty and differ between languages; Arabic snapshot mirrors button order | snapshot |

Windows live check (Start menu build, a click opens the default browser): left to the owner.

## Out of scope
- Release notes, credits, licence text viewer, macOS.

## Review log
-
