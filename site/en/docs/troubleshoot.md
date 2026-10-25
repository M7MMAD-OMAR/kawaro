# Troubleshooting

- **An action is missing:** its tool is not installed. Run `kawaro doctor`.
- **Windows warns "Windows protected your PC":** the build is unsigned. [Verify the file](https://kawaro.sbarah.com/en/download/#verify), then choose More info and Run anyway.
- **The hive opens without keyboard focus (GNOME, KDE):** focus tests on real desktops are not finished. Report it with your desktop and version.
- **Nothing happens on right click:** run `kawaro ping`. If the service does not answer, run `kawaro daemon` and try again.
- **Updates:** set `update_check = false` to stop the daily check ([settings](settings.md)).

Report a problem on [GitHub](https://github.com/M7MMAD-OMAR/kawaro/issues).
