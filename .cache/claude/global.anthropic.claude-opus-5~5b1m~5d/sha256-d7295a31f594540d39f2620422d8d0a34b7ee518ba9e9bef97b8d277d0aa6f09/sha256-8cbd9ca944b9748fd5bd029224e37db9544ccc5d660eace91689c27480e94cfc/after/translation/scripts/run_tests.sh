#!/usr/bin/env bash
# Full differential verification run (Phases B, C, D) over every build
# configuration declared in Cargo.toml.
#
# RUST_TEST_THREADS=1 is mandatory: the harness captures the process-wide stdout
# file descriptor, which cannot be shared by concurrent test threads.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(cd .. && pwd)"

echo "== building the C shared library =="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )

# Enumerate feature combinations from Cargo.toml. With no [features] section the
# only configuration is the default one, which is also what
# --no-default-features selects.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml
)

COMBOS=("")
if ((${#FEATURES[@]} > 0)); then
  echo "features found: ${FEATURES[*]}"
  # Power set of the declared features.
  n=${#FEATURES[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
else
  echo "no [features] in Cargo.toml -> single (default) configuration"
fi

status=0
for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    label="default / --no-default-features"
    flags=(--no-default-features)
  else
    label="--no-default-features --features $combo"
    flags=(--no-default-features --features "$combo")
  fi
  echo
  echo "=================================================================="
  echo "== configuration: $label"
  echo "=================================================================="
  cargo build --release --offline "${flags[@]}"
  cargo check --release --offline "${flags[@]}"
  if ! RUST_TEST_THREADS=1 cargo test --release --offline "${flags[@]}"; then
    echo "FAILED under: $label"
    status=1
  fi

  echo "-- symbol diff (C vs Rust) under $label --"
  diff <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/release/libdriver.so      | awk '{print $NF}' | sort) \
    && echo "symbol diff: EMPTY (parity)" || { echo "symbol diff NOT EMPTY"; status=1; }
done

echo
if ((status == 0)); then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit $status
