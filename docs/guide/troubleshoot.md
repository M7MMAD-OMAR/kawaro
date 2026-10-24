[English](troubleshoot.md) | [العربية](troubleshoot.ar.md)

# Troubleshooting

- **An action is missing:** its tool is not installed. Run `sbarhive doctor`.
- **Windows warns "Windows protected your PC":** the build is unsigned. [Verify the file](site:download#verify), then choose More info and Run anyway.
- **The hive opens without keyboard focus (GNOME, KDE):** focus tests on real desktops are not finished. Report it with your desktop and version.
- **Nothing happens on right click:** run `sbarhive ping`. If the service does not answer, run `sbarhive daemon` and try again.
- **Updates:** set `update_check = false` to stop the daily check ([settings](settings.md)).

Report a problem on [GitHub](https://github.com/M7MMAD-OMAR/sbarhive/issues).
