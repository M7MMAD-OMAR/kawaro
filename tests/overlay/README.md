# Overlay tests | اختبارات الطبقة

## العربية

اختبارات S-018: الخلية طبقةً (zwlr_layer_shell_v1) ونافذةً عادية. كل سكربت يشغّل مؤلِّفاً خاصاً بلا شاشة (KWin أو GNOME Shell) ببيئة ومجلدات مؤقتة خاصة، ولا يلمس سطح المكتب الحي.

ابنِ أولاً:

```
cargo build --release
cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml
```

ثم شغّل الكل بأمر واحد:

```
tests/overlay/run.sh
```

يشغّل `run.sh` بالترتيب: `docs.sh`، ثم `kwin.sh` بالوضع الافتراضي (الطبقة) وبكل وضع آخر (`window` و`input` و`token` و`a11y` و`drop` و`scheme` و`1.5` و`anchor`)، ثم `gnome.sh` و`focus.sh` و`budget.sh`. يطبع سطراً لكل خطوة، ومخرجات كل خطوة في `tests/overlay/out/run-<الخطوة>.txt`. التشغيل الكامل يأخذ وقتاً ويحمّل الجهاز، فلتشغيل جزء واحد استدعِ سكربته مباشرةً (طريقة الاستعمال في رأس كل سكربت).

## English

S-018 tests: the hive as a zwlr_layer_shell_v1 overlay and as an ordinary window. Each script runs a private headless compositor (KWin or GNOME Shell) with its own throwaway environment and folders, and never touches the live desktop.

Build first (the two `cargo build` lines above), then run everything with the one command above, `tests/overlay/run.sh`.

It runs, in order: `docs.sh`, then `kwin.sh` in the default (layer) mode and in every other mode (`window`, `input`, `token`, `a11y`, `drop`, `scheme`, `1.5`, `anchor`), then `gnome.sh`, `focus.sh` and `budget.sh`. It prints one line per step and keeps each step's output in `tests/overlay/out/run-<step>.txt`. A full run is long and loads the machine; to run one part, call its script directly (usage is in each script's header).
