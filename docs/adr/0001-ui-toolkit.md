# ADR 0001: UI toolkit and renderer for the hive

Status: proposed (waiting for the focus test on real GNOME)
Date: 2026-10-19

## Context

The spike in `spikes/hive/` built the hive (19 hexagonal cells, a transparent frameless window) with Slint 1.18.1 on winit and measured it in private headless compositors (KWin 6.7.5 virtual, GNOME Shell 50.5 headless) with an isolated profile, never touching the live desktop. Each number is the median of 7 or 8 runs.

| Compositor | Renderer | Cold start to first frame | Through the service (IPC to first frame) | Idle service memory | Focus |
| --- | --- | --- | --- | --- | --- |
| KWin | femtovg (OpenGL) | 257 ms | 50 ms | 144 MB | yes |
| KWin | software | 129 ms | 41 ms | 28 MB | yes |
| GNOME | femtovg (OpenGL) | 172 ms | 55 ms | 129 MB | no (inconclusive) |
| GNOME | software | 99 ms | 118 ms | 28 MB | no (inconclusive) |

Measurement notes:

- The service median leaves out the first show after the service starts, which reported no frame within 3 s in any run (finding 8).
- Headless compositors render in software (llvmpipe), so femtovg looks worse here than on a machine with a GPU.
- All numbers come from a fast machine, not a low end machine (4 GB, two cores). Limiting the spike to two fast cores (software, KWin) gave 101 ms cold and 29 ms through the service, which does not represent a slow CPU. A real low end measurement is still needed.

## Findings

1. **Slint fits the hive.** Path drawing, hexagon shaped hit testing, joined Arabic and mirrored layout all work. Screenshots in `spikes/hive/out/`.
2. **The background service is required in practice.** With the software renderer and a clean profile, cold start is under 150 ms (99 and 129 ms), but not with femtovg (172 and 257 ms), not with a font heavy profile (about 300 ms), and probably not on a weak machine. Through the service it stayed between 41 and 118 ms in every case.
3. **The software renderer is the choice.** It meets the memory budget (28 MB against the 40 MB limit in S-001); femtovg is three times over because of OpenGL. The hive is small (393 by 362), so software rendering costs almost nothing.
4. **Fonts are the largest hidden cost.** On a machine with a very large font set, memory with the real profile rose to 56 MB (software) and 173 MB (femtovg), and cold start grew by about 200 ms. Decision: bundle IBM Plex Sans Arabic and look into scanning fewer system fonts.
5. **On Wayland the surface is destroyed on hide.** Slint destroys the winit window on `hide()` and recreates it on `show()`. The numbers above include that, so no keep alive trick is needed.
6. **The activation token can be passed.** The `with_winit_window_attributes_hook` hook lets the service pass the xdg-activation token from the IPC request. Corrected 2026-10-20, see Amendments.
7. **Focus on GNOME is unresolved and is the largest risk.** In headless GNOME no window got focus (0 of 8), even on a plain cold start; the likely cause was the missing keyboard. Headless KWin gave focus without a token. If this held in a real session, Esc would not arrive and close on focus loss would not work (it needs prior focus), so the hive would stay open until clicked. A final verdict needs a real GNOME session with a token from Nautilus. Results of 2026-10-20 in Amendments.
8. **The first show after the service starts is unmeasured and unexplained.** It took over 3 s in every run, about 8 s in a traced run. Hypothesis, not proven: waiting for the accessibility bus (AT-SPI), whose activation failed inside the isolated session bus. This is the first show a user sees after login, so it must be measured in a real session before the decision is accepted.

## Decision

- Continue with Slint and the hexagonal hive.
- Default renderer: software. femtovg or skia are reconsidered only if we need drawing it cannot do.
- The background service is mandatory; cold start is a fallback path.
- The font is bundled in the program.

## Consequences

- Done: live run on Hyprland (2026-10-19). Cold first frame at 226 ms with the full profile and fonts, the window got focus (`focused=true`) and closed on switching to another window (`focus_lost`); a second run (209 ms) closed with Esc or a click outside the cells (`dismissed`). Cold start above 150 ms confirms the service is required.
- Done: headless GNOME Shell 50.5 (2026-10-20, S-013). A token from a GTK 4 window, using the same call as the Nautilus script, gives focus with no "ready" notification, even under the strict policy.
- [ ] GNOME VM: the real Nautilus script requests and passes a token; check focus with no "ready" notification.
- [ ] KDE VM: the same test through Dolphin.
- [ ] Measure the first show again with a real accessibility bus.

## Amendments

### 2026-10-20 (S-013): finding 6 corrected

The hook runs only when the window adapter is created, that is when the component is created, not when the winit window is recreated by `show()` after `hide()`. So no token was ever sent. Fix: `present` builds a new hive component when it receives a token. Details in `backlog/S-013-focus-gnome-kde.md`.

### 2026-10-20 (S-013, `tests/focus/`): finding 7 results

The missing keyboard was not the only cause. Headless GNOME starts in the activities overview, where no window has focus. With the overview hidden and a virtual keyboard from Mutter's RemoteDesktop interface, the hive got focus under GNOME's default policy (smart), with or without a token, and Esc closed it. Under the strict policy it got focus only with a real token from a GTK 4 window; with no token or a forged one the "\"SbarHive\" is ready" notification showed and the hive had no keyboard. KWin at Extreme gave the same result; at Low and High it gave focus in every case. These results are after the finding 6 fix; before it the hive never got focus with a real token under a strict policy. Still open: a real session with a real file manager (Nautilus, Dolphin), and why the first trial show in GNOME did not get focus when no other window existed.
