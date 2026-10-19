# Hive spike

Throwaway experiment for phase 0a. Results and the decision live in
`docs/adr/0001-ui-toolkit.md`. Nothing here is merged into `crates/` as is.

## Build

```bash
cargo build --release
cargo test --release
```

## Try it live

```bash
SPIKE_RENDERER=software ./target/release/hive-spike run --lang ar
```

`--lang en` for English, `--light` for the light theme. Esc, a click outside a
cell, or switching to another window closes it.

Daemon path, the way a file manager integration will use it:

```bash
SPIKE_RENDERER=software ./target/release/hive-spike daemon --lang ar
```

```bash
./target/release/hive-spike show
```

## Measure without touching the desktop

`measure.sh kwin|gnome [runs]` starts a private headless compositor with a
throwaway HOME and its own session bus, runs cold and daemon timings, reads
idle memory, and writes snapshots to `out/`. `SPIKE_RENDERER` picks
`software` or `femtovg`.
