# alt-portal spike: Alt-only GlobalShortcuts on the real portals

Spike source, restored 2026-11-15. It is the measurement source for the
disproven Alt-only GlobalShortcuts mechanism, not a product path.

## Provenance

- Restored from the branch `spike/alt-portal`, commit `5b6a552`
  ("spike: test Alt-only global shortcuts", 2026-10-26). The nine source
  files are byte identical to that commit.
- The spike copies the daemon portal client with
  `PREFERRED_TRIGGER = "ALT"` instead of the shipped `"LOGO+z"`, adds its
  tests, and adds drop scripts that press Alt instead of Super+Z.
- No text was normalized during the restore: the files already contain no em
  dash, no en dash and no guillemet.

## What the measurement found

Recorded in `backlog/S-075-alt-drag-by-default.md` and `docs/PLAN.md`
(measured facts, 2026-10-26): on KWin 6.7.5 with xdg-desktop-portal-kde
6.7.5 and on GNOME Shell 50.6 with xdg-desktop-portal-gnome 50.0, an
Alt-only shortcut was accepted by the BindShortcuts exchange but produced
zero Activated and Deactivated signals and zero working Alt-drag
conversions. The Super+Z control passed the full real-drag path on both.

That is why no story may claim Alt through the GlobalShortcuts portal on
KDE or GNOME, and why the shipped portal client keeps
`PREFERRED_TRIGGER = "LOGO+z"`.

## Limitations

- Spike source. It is outside the Cargo workspace, it is not built, tested or
  linted by CI, and it is not part of the agent chain for product work.
- Restoring this source performs no measurement. The results above are the
  2026-10-26 results; nothing was re-measured during the restore.
- The native Alt paths, S-075b for KDE and S-075d for GNOME, are still open
  in `docs/PLAN.md`. The GNOME half has spike source in
  `spikes/gnome-alt-extension/`. The KDE half has no source at all: it was
  never written.
- Hyprland compositor binds were outside this spike and count only when the
  private Hyprland harness records them.
