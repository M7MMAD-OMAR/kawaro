# gnome-alt-extension spike: Alt_L press and release from a GNOME Shell extension

> **Provenance, restored 2026-11-15.** These seven source files were restored
> into `main` from the detached worktree `spikes/gnome-alt-extension`
> (HEAD `50715f0`), where they had been untracked. They are byte identical to
> the verified backup archive
> `worktree-payloads/spikes-gnome-alt-extension/gnome-alt-extension-source.tar.gz`
> in `~/kawaro-consolidation-2026-11-15`. That worktree was parked, not
> deleted, at `~/kawaro-consolidation-2026-11-15/parked-worktrees/gnome-alt-extension`.
> No text was normalized during the restore: the files already contain no em
> dash, no en dash and no guillemet.

Spike for S-075 (Alt then drag works out of the box). The real
GlobalShortcuts portal rejects Alt-only on GNOME 50.6 and KWin 6.7.5 while
Super+Z passes (`alt-portal-spike.md` in the checkpoint runs). The maintainer
requires Alt as the primary drag key, so this spike asks whether a native
GNOME mechanism can capture `Alt_L` press and release globally, including
during a real GTK file drag, and feed the daemon's existing long press IPC
(`kawaro drop --hold` on the press, `kawaro drop --release` on the release).

Everything runs in one private headless GNOME session
(`gnome-shell --headless --wayland --no-x11 --unsafe-mode`) on a private
session bus (`dbus-run-session`) with a throwaway HOME, XDG dirs and runtime
dir. The live desktop and live settings are never touched.

## Files

- `probe.js`: Phase A, installed with Shell.Eval only. Logs every watched
  key press and release from three mechanisms side by side:
  `global.stage` captured-event, `global.stage` key-press-event /
  key-release-event, and `global.display.grab_accelerator` +
  `accelerator-activated` / `accelerator-deactivated`. `global._sbh.reset()`
  empties the log, `global._sbh.remove()` disconnects it again.
- `ext/kawaro-alt@kawaro.test/`: Phase B, the minimal real extension shape
  (metadata.json + extension.js, GNOME 45+ ESM). Same three mechanisms, and
  forwarding: the first captured press of a watched key runs the configured
  hold command, the next captured release runs the release command. The
  commands and log path come from `$XDG_RUNTIME_DIR/kawaro-ext.conf`, so the
  product shape is the same commands the Hyprland bind runs.
- `input_alt.py`: tests/drop/input_gnome.py with `Alt_L`, `Alt_R` and `F9`
  keysyms added, driving a private headless GNOME through Mutter's
  RemoteDesktop API on the private bus only.
- `run.sh`: the harness (phases A to C), reusing tests/drop/lib.sh.
- `heavy.sh`: the wrapper for
  `flock /home/sbarah/.cache/kawaro-heavy.lock systemd-run --user --scope
  -p MemoryMax=6G -p CPUQuota=200%`.

## Questions answered by run.sh

1. Can a real extension capture `Alt_L` press and release globally,
   including during a real GTK file drag? (Phase A with Eval only first,
   then Phase B with the extension.) F9 is the conventional positive
   control key in every case.
2. Do hold_ms (400 ms) and the release reach the daemon? The extension
   runs `kawaro drop --hold` / `kawaro drop --release`, the same IPC the
   Hyprland bind runs: the hive must show 400 ms after the press, a 150 ms
   tap must show nothing (the release cancels the gate), and the release
   must close a shown hive, with and without a drag.
3. Does the drag stay copy-only with an unchanged original?
   `drag-end deleted=False`, the original's sha256 kept, and the daemon
   trace's `wl_data_offer.set_actions` only ever (1, 1) or (0, 0).
4. Can install and enable work without a shell restart on Wayland? The
   extension is copied into the throwaway
   `XDG_DATA_HOME/gnome-shell/extensions` after the shell started and
   enabled live; the exact API that worked and every raw D-Bus reply are
   recorded (`gdbus-*.txt`), with the same gnome-shell PID throughout.

## Raw logs

Beside the report:
`/home/sbarah/.hermes/checkpoint-runs/kawaro-stable-20261026/gnome-alt-extension-logs/`.

## Limitations

- Spike source. It is outside the Cargo workspace, it is not built, tested or
  linted by CI, and it is not part of the agent chain for product work.
- Restoring this source performs no measurement. The harness and the
  questions below are the ones written on 2026-10-26; nothing was re-run
  during the restore, and this file certifies no native GNOME Alt path.
- The native Alt paths, S-075d for GNOME and S-075b for KDE, are still open
  in `docs/PLAN.md`. The KDE half has no source at all: it was never written.
- The run logs live outside the repository, at the path above, so they are not
  part of this tree.
