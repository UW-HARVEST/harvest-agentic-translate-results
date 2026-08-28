#!/usr/bin/env bash
# Build the C library and the Rust cdylib, then run the differential test suite
# for every Cargo feature combination and for both Rust build profiles.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"

# --- C ground truth ---------------------------------------------------------
if [[ ! -f "$ROOT/c_src/build/libdriver.so" ]]; then
  echo "=== building C shared library"
  (cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
    && cmake --build . >>/tmp/cmake.log 2>&1) || { tail -30 /tmp/cmake.log; exit 1; }
fi

mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)
combos=("")
for f in "${FEATURES[@]}"; do
  new=()
  for c in "${combos[@]}"; do
    new+=("$c")
    if [[ -z "$c" ]]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  combos=("${new[@]}")
done

status=0
for c in "${combos[@]}"; do
  for profile in release debug; do
    label="${c:-<none>}"
    echo "=== features=$label profile=$profile"
    relflag=(); [[ $profile == release ]] && relflag=(--release)
    # Build the cdylib under test, then load *that* .so from the tests.
    timeout 600 cargo build "${relflag[@]}" --no-default-features --features "$c" >/tmp/bt.log 2>&1 \
      || { tail -20 /tmp/bt.log; status=1; continue; }
    DRIVER_RUST_SO="$PWD/target/$profile/libdriver.so" \
      timeout 600 cargo test --release --no-default-features --features "$c" 2>&1 \
      | grep -E 'test result|FAILED|panicked|error' || status=1
  done
done
exit $status
