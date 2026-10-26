# S-077 The hive opened from the right-click menu appears at the pointer, not stuck to the top edge
Status: approved (2026-10-26, manager). Needs a short spike on KDE and GNOME first.

## Story
As a user of the right-click menu, I want the hive to appear where I clicked, as the drag path does, so that it is not glued to the top of the screen.

## Background
`kawaro-open-selection` passes no position, so the hive uses the default (top). On Hyprland `hyprctl cursorpos` gives the pointer; on KDE the Dolphin service menu runs without a pointer; on GNOME the window path is centred by design (CLAUDE.md).

## Acceptance criteria
- AC1: Hyprland: the menu path passes `--at` from the compositor pointer and the hive centres on it, adapting to edges (as S-019).
- AC2: KDE: a spike records what gives the pointer position to a service-menu process (KWin scripting, `org.kde.KWin` DBus `queryWindowInfo`, or the activation token surface); the story is split by the result. If nothing works, the hive is centred on the screen of the focused window, never stuck to the top.
- AC3: no path leaves the hive touching the top edge unless the pointer is there.
- AC4: GNOME stays centred.

## Review log
(empty)
