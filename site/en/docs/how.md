# How it works

One service shows the interface. A thin command line client asks it to open the hive.

![From right click to result](https://hive.sbarah.com/diagrams/flow.en.light.svg)

## The pieces

![Processes and crates](https://hive.sbarah.com/diagrams/overview.en.light.svg)

- `sbarhived` holds the hive, the queue, the tray and the engines.
- FFmpeg, LibreOffice and Ghostscript run as separate processes, killed with their whole tree on cancel.
- The original file is never changed: the result is written to a temporary file and then renamed.

## Add a format

![TOML plus an adapter](https://hive.sbarah.com/diagrams/extend.en.light.svg)

## Where the hive is drawn

![Layer path or window path](https://hive.sbarah.com/diagrams/surface.en.light.svg)

## Updates

![Signed updates](https://hive.sbarah.com/diagrams/updates.en.light.svg)
