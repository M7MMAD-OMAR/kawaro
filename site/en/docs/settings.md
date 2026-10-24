# Settings

The file is `config.toml`: `~/.config/sbarhive/` on Linux, `%APPDATA%\SbarHive\` on Windows. Every key is optional, unknown keys are ignored, and a broken file falls back to the defaults.

| Key | Values | Default |
| --- | --- | --- |
| `language` | `auto`, `ar`, `en` | `auto` |
| `theme` | `auto`, `light`, `dark` | `auto` |
| `output` | `beside`, `subfolder` | `beside` |
| `output_folder` | folder name for `subfolder` | `SbarHive` |
| `quality` | `light`, `balanced`, `max` | `balanced` |
| `open_folder_when_done` | `true`, `false` | `false` |
| `notify` | `true`, `false` | `true` |
| `hive_surface` | `auto`, `layer`, `window` | `auto` |
| `hold_ms` | 100 to 2000 | `400` |
| `portal_shortcut` | `true`, `false` | `true` |
| `shortcut` | Windows hotkey | `Ctrl+Alt+H` |
| `update_check` | `true`, `false` | `true` |
| `update_channel` | `stable`, `beta` | `stable` (`beta` in a beta build) |
| `update_mode` | `ask`, `auto` | `ask` |
