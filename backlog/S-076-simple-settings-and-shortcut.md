# S-076 Settings without technical words, with a shortcut picker (Alt then drag by default)
Status: approved (2026-10-26, manager)

## Story
As an ordinary user, I want a settings window with only plain choices and a way to change the drag key, so that I never meet a technical term and can still pick my own shortcut, with Alt as the default.

## Decisions (maintainer's words, 2026-10-26)
1. Remove everything technical from the window. Reviewed against the current window: "Register the drop shortcut with the desktop" goes (S-075 does it automatically); "Folder name" shows only when "In a subfolder" is chosen; wording is plain Arabic and English ("Open the folder when done" stays). Anything else a normal user cannot explain in one sentence is removed or moves to `config.toml`.
2. A new row "Drag key": shows `Alt` by default and a "Change" button; the user presses the new key or combination and it is saved (Alt, Ctrl, Super, Shift, or a combination, never a plain letter alone). A "Reset to Alt" button. The helper text says what it does: "Hold this key while dragging files to open the hive".
3. The saved key is applied to the desktop binding by the daemon (Hyprland snippet, portal) without a restart, using the S-075 machinery. A conflict with a known desktop shortcut shows a plain message.
4. Material 3 components only, tokens only, RTL, labels on every control, Arabic and English strings.

## Acceptance criteria
- AC1: the window has no row for portal or shortcut registration, and no string in either language contains: portal, daemon, registry, config, IPC, token.
- AC2: the "Drag key" row shows Alt on a fresh config, in both languages and both directions.
- AC3: choosing a new key saves `drop_key` in `config.toml` and the live binding changes within 2 s (Hyprland harness: the old key no longer opens the hive, the new one does).
- AC4: a plain letter alone is refused with a message; Escape cancels the capture.
- AC5: "Reset to Alt" restores the default.
- AC6: snapshots updated (EN, AR, light, dark), design lint clean.

## Review log
(empty)
