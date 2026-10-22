[English](mobile-idea.md) | [العربية](mobile-idea.ar.md)

# Mobile version idea (Android first)

Status: an idea only. No work before Linux (0.4, 0.5) and Windows (0.6) are done. Recorded 2026-10-22. The owner has no Mac, so iOS is out of scope for now.

## Principle

Not a shrunk desktop app. The same core (`core`, `engines`) with a different experience: **no app is opened to convert; the hive appears inside the system, over whatever the user is doing.**

## System entry points, by priority

1. **Share sheet:** the first target. A translucent activity shown as a bottom sheet over the gallery or file manager, so it feels part of the system. Pick an action in the hive, the job starts, and the user is back where they were.
2. **Open with (`ACTION_VIEW`) and multi select commands** from the file manager.
3. **App shortcuts** on long press: "convert the last photo", "compress camera photos".
4. **Quick Settings tile:** opens the hive on the last copied or captured item.
5. **Home screen widget:** optional, for repeated one tap actions. Late priority.
6. **Notification as the progress UI:** a foreground service with progress, then Share, Open and Undo buttons when done.

Rejected for the first version: an Accessibility service or a draw over everything overlay (`SYSTEM_ALERT_WINDOW`). Sensitive permissions that stores reject or users distrust, and against the project's privacy spirit.

## Experience

- One tap picks the action, no questions before it runs. Fine options come after the choice, with a swipe up.
- Large cells (48 dp or more), one or two rings, and the full name always shown in the centre cell, since there is no hover.
- Light haptics on selection, short motion that respects reduced motion, and the same visual rules (no shadows, no gradients, values from `design/tokens.json`).
- The hive appears at once and work runs in the background. Nothing blocks the user.
- Output: a folder the user grants once (`ACTION_OPEN_DOCUMENT_TREE` with a persisted permission), keeping the "new copy next to the original" rule; otherwise save to the gallery or share directly.
- Arabic, English and RTL from day one.

## Technical notes

- Targets: `aarch64-linux-android` first, armv7 if needed.
- `core` is nearly ready: the `XDG_*` paths in `ipc.rs` and `config.rs` need replacing.
- `engines`: images and PDF only. Video later with hardware codecs (MediaCodec). LibreOffice is excluded.
- `ui`: Slint on Android means `backend-android-activity` instead of `backend-winit`, plus a new integration layer. Slint on iOS is not checked.
- `libwebp` is a C library: it needs the NDK and `cargo-ndk`.
- The native front (translucent activity, notifications, foreground service, share receiver) is thin Kotlin calling the Rust library over JNI.
- Distribution: F-Droid first (fits GPLv3). Google Play possibly later, after a licence review.

## Not verified

- Whether a translucent activity can start and show within 150 ms on mid range devices.
- Memory and thermal limits when converting a large batch of images.
- Whether Slint draws transparency over another app without issues on every Android version.
- Limits on starting activities from the background on Android 10 and later.

## First step when the time comes

A spike in `spikes/mobile/` (not merged): build `core` and `engines` for Android, plus a translucent activity that receives a photo from the gallery, shows the hive and converts it to WEBP on a real device. The result becomes an ADR, then the plan is decided.
