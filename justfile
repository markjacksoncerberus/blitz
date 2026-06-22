
## Build/lint commands

check:
  cargo check --workspace

clippy:
  cargo clippy --workspace

fmt:
  cargo fmt --all

small:
  cargo build --profile small -p counter --no-default-features --features cpu,system_fonts

## WPT test runner

wpt *ARGS:
  cargo run --release --package wpt {{ARGS}}

## WPT compliance loop (see wpt/triage.sh and .claude/commands/wpt.md)

# Build + run the suite (default: flexbox + grid), saving a parsed report.
wpt-run *SUITES:
  ./wpt/triage.sh run {{SUITES}}

# Rank the most tractable failures to fix next (near-pass checkLayout tests + clusters).
wpt-targets *LIMIT:
  ./wpt/triage.sh targets {{LIMIT}}

# Show the exact failing assertions ("expected X got Y") for a test.
wpt-errors SUBSTR:
  ./wpt/triage.sh errors {{SUBSTR}}

# Freeze the current results under a label (e.g. before / after a change).
wpt-snapshot LABEL:
  ./wpt/triage.sh snapshot {{LABEL}}

# Diff two snapshots: net improvements and regressions.
wpt-compare BEFORE AFTER:
  ./wpt/triage.sh compare {{BEFORE}} {{AFTER}}

browser *ARGS:
  cargo run --release --package browser --features log_frame_times,log_phase_times {{ARGS}}

browser-with-perf:
  cargo run --release --package browser --features log_frame_times,log_phase_times

browskia:
  cargo run -rp browser --no-default-features --features skia,floats,incremental,cookies,cache,log_frame_times,log_phase_times

## Browser

screenshot *ARGS:
  cargo run --release --example screenshot {{ARGS}}

open *ARGS:
  cargo run --release --package rdme --features log_frame_times,log_phase_times {{ARGS}}

openskia *ARGS:
  cargo run --release --package rdme --no-default-features --features skia,comrak,floats,incremental,log_frame_times,log_phase_times {{ARGS}}

opencpu *ARGS:
  cargo run --release --package rdme --no-default-features --features cpu,comrak,floats,incremental,log_frame_times,log_phase_times {{ARGS}}

dev *ARGS:
  cargo run --package rdme --features log_frame_times,log_phase_times {{ARGS}}

incr *ARGS:
  cargo run --release --package rdme --features incremental,comrak,floats,log_frame_times,log_phase_times {{ARGS}}

cpu *ARGS:
  cargo run --release --package rdme --no-default-features --features cpu,comrak,incremental,floats,log_frame_times,log_phase_times {{ARGS}}

hybrid *ARGS:
  cargo run --release --package rdme --no-default-features --features hybrid,comrak,incremental,floats,log_frame_times,log_phase_times {{ARGS}}

skia *ARGS:
  cargo run --release --package rdme --no-default-features --features skia,comrak,incremental,floats,log_frame_times,log_phase_times {{ARGS}}

skia-pixels *ARGS:
  cargo run --release --package rdme --no-default-features --features skia-pixels,comrak,floats,incremental,log_frame_times,log_phase_times {{ARGS}}

skia-softbuffer *ARGS:
  cargo run --release --package rdme --no-default-features --features skia-softbuffer,comrak,floats,incremental,log_frame_times,log_phase_times {{ARGS}}

## 7GUIs

seven_guis *ARGS:
  cargo run --release --package seven_guis --bin seven_guis_native {{ARGS}}

## TodoMVC commands

todomvc *ARGS:
  cargo run --release --package todomvc --bin todomvc_native {{ARGS}}

todoskia *ARGS:
  cargo run --release --package todomvc --bin todomvc_native {{ARGS}} --no-default-features --features skia

todoandroid *ARGS:
  export CARGO_APK_RELEASE_KEYSTORE="$HOME/.android/debug.keystore"
  export CARGO_APK_RELEASE_KEYSTORE_PASSWORD="android"
  cargo apk run --lib --no-default-features --features skia -p todomvc

counterandroid *ARGS:
  export CARGO_APK_RELEASE_KEYSTORE="$HOME/.android/debug.keystore"
  export CARGO_APK_RELEASE_KEYSTORE_PASSWORD="android"
  cargo apk run --lib --no-default-features --features skia -p counter

## WASM

wasm-build APP *ARGS:
  cd examples/{{APP}} && trunk build --release --public-url ./ {{ARGS}}

wasm-serve APP *ARGS:
  cd examples/{{APP}} && trunk serve --release --public-url ./ {{ARGS}}

## Ops

bump *ARGS:
  cargo run --release --package bump {{ARGS}}