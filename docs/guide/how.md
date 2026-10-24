[English](how.md) | [العربية](how.ar.md)

# How it works

One service shows the interface. A thin command line client asks it to open the hive.

![From right click to result](diagram:flow)

## The pieces

![Processes and crates](diagram:overview)

- `sbarhived` holds the hive, the queue, the tray and the engines.
- FFmpeg, LibreOffice and Ghostscript run as separate processes, killed with their whole tree on cancel.
- The original file is never changed: the result is written to a temporary file and then renamed.

## Add a format

![TOML plus an adapter](diagram:extend)

## Where the hive is drawn

![Layer path or window path](diagram:surface)

## Updates

![Signed updates](diagram:updates)
