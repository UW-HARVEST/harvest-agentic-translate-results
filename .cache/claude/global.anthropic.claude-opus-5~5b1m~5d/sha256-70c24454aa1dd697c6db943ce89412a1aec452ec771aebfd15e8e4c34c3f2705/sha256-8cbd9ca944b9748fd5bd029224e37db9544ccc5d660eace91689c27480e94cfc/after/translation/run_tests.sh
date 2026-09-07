#!/usr/bin/env bash
# Build the C reference library and the Rust cdylib, then run the differential
# test suite across every Cargo feature combination.
#
# `--test-threads=1` is REQUIRED: the `driver` tests redirect file descriptor 1
# to capture printf output, and fd 1 is process-global, so libtest's own
# progress output would otherwise land in the capture.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

CARGO_FLAGS=(--offline)

echo "== building C reference library =="
mkdir -p "$root/c_src/build"
(
  cd "$root/c_src/build"
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
)
ls -l "$root/c_src/build/libStaticAlias.so"

cd "$here"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml (the [features] table).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

declare -a COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "== no [features] in Cargo.toml: exactly one build configuration =="
  COMBOS+=("<default>")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo+="${FEATURES[$i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
  COMBOS+=("<default>")
fi

status=0
for combo in "${COMBOS[@]}"; do
  echo
  echo "=============================================================="
  echo "== feature combination: ${combo:-<none>}"
  echo "=============================================================="
  if [ "$combo" = "<default>" ]; then
    featargs=()
  elif [ -z "$combo" ]; then
    featargs=(--no-default-features)
  else
    featargs=(--no-default-features --features "$combo")
  fi

  cargo check "${CARGO_FLAGS[@]}" "${featargs[@]}" || { status=1; continue; }
  # The .so under test must exist before the integration tests dlopen it.
  cargo build "${CARGO_FLAGS[@]}" --release "${featargs[@]}" || { status=1; continue; }
  cargo test "${CARGO_FLAGS[@]}" --release "${featargs[@]}" -- --test-threads=1 \
    || status=1
done

echo
if [ "$status" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES -- see above"
fi
exit "$status"
