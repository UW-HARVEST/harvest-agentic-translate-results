#!/usr/bin/env bash
# Phase D — run the whole verification under EVERY feature combination.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so if a
# [features] section is ever added this script picks it up automatically and the
# "all combinations" claim keeps holding.
set -euo pipefail
cd "$(dirname "$0")"

mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "declared non-default features: $N ${FEATURES[*]:-(none)}"

echo "== building C reference =="
(cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)

run_combo() {
  local label="$1"; shift
  echo
  echo "=================================================================="
  echo "COMBO: $label"
  echo "=================================================================="
  cargo build --release "$@"
  cargo check --all-targets "$@"
  cargo test --release "$@"
  # Symbol parity must hold for every combination, not just the default.
  local c_so rust_so
  c_so=$(ls ../c_src/build/*.so | head -1)
  rust_so=target/release/libgjk_lib.so
  diff <(nm -D --defined-only "$c_so"     | awk '$2=="T"{print $3}' | sort) \
       <(nm -D --defined-only "$rust_so"  | awk '$2=="T"{print $3}' | sort) \
    && echo "symbol parity OK for [$label]"
}

# The default build.
run_combo "default"
# Explicitly featureless.
run_combo "--no-default-features" --no-default-features

# Every subset of the declared features (2^N), if there are any.
if [ "$N" -gt 0 ]; then
  for ((mask=0; mask<(1<<N); mask++)); do
    combo=()
    for ((i=0; i<N; i++)); do
      if (( (mask >> i) & 1 )); then combo+=("${FEATURES[$i]}"); fi
    done
    joined=$(IFS=,; echo "${combo[*]:-}")
    run_combo "--no-default-features --features ${joined:-<empty>}" \
      --no-default-features ${joined:+--features "$joined"}
  done
  # And everything at once, on top of the defaults.
  run_combo "--all-features" --all-features
fi

echo
echo "ALL FEATURE COMBINATIONS PASSED"
