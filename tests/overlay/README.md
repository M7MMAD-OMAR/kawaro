# Overlay tests

S-018 tests: the hive as a `zwlr_layer_shell_v1` overlay and as an ordinary window. Each script runs a private headless compositor (KWin or GNOME Shell) with its own throwaway environment and folders, and never touches the live desktop.

Build first:

```
cargo build --release
cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml
```

Then run everything with one command:

```
tests/overlay/run.sh
```

`run.sh` runs, in order: `docs.sh`; `kwin.sh` in the default (layer) mode and in every other mode (`window`, `input`, `token`, `a11y`, `drop`, `scheme`, `1.5`, `anchor`, `outputs`; the last runs KWin with two virtual outputs for the S-019 same output checks); then `gnome.sh`, `focus.sh` and `budget.sh`. It prints one line per step and keeps each step's output in `tests/overlay/out/run-<step>.txt`.

A full run is long and loads the machine; run it inside the scope from `CLAUDE.md` ("Running heavy work"). To run one part, call its script directly; usage is in each script's header.
