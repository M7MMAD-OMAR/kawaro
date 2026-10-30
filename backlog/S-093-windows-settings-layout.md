# S-093 Windows settings window: size, wide layout and layout jumps
Status: superseded by S-094 (2026-10-29): D1 to D4 are covered by S-094 AC3 to AC6.

## Context
Live test of the S-090 build on Windows 11 (a Windows 11 virtual machine, per-user MSI built from main at f5242b6, 2026-10-28). Verified working: the install, the «Kawaro | كوارو» entry in Start with the icon, the Arabic RTL welcome window, the Updates section with the installed version `0.4.0`, "Check for updates" (state «جارٍ البحث عن تحديث.» then «نسختك محدّثة.»), and "Update now" disabled when up to date.

## Defects found
- D1: the welcome window opens about 590 by 470 pixels, so the Updates section sits several screens below the fold and needs many wheel steps. Language and Theme alone fill the first view.
- D2: maximized, the switches sit at the far left and their labels at the far right (RTL), a gap of more than 1500 pixels, so a label and its switch are hard to pair. The settings column needs a maximum width and centring.
- D3: the state line appears above the buttons when a check starts and pushes the rows below it down (layout jump); reserve its height.
- D4: the last choice row ("Install updates") is cut at the bottom edge of the scroll area while more content remains.

## Acceptance criteria
- AC1: the default welcome window height shows Language, Theme and the start of the next section without clipping, and a section heading is never cut in half by the footer.
- AC2: the settings column has a maximum width token and a label and its switch are never farther apart than that width.
- AC3: starting a check changes no row position (the state line height is reserved).
- AC4: screenshots on Windows 11 in Arabic and English at default and maximized size, owner approved.

## Out of scope
- New settings or update logic.

## Review log
- Findings recorded from the live run; no code changed.
