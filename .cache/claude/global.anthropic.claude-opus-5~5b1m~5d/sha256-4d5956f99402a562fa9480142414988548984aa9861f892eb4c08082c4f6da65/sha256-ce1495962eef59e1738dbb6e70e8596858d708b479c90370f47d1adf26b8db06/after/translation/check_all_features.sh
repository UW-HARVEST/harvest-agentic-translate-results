#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# Feature names are extracted from Cargo.toml's [features] table; the power set
# is enumerated automatically, so adding a feature later needs no edits here.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

CARGO_FLAGS=(--offline)

# ---- 1. build the C .so ---------------------------------------------------
if ! ls ../c_src/build/lib*.so >/dev/null 2>&1; then
  echo "== building the C shared library"
  ( mkdir -p ../c_src/build && cd ../c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "FAIL: C build"; exit 1; }
fi

# ---- 2. enumerate the feature power set ----------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "== features declared in Cargo.toml: ${n} (${FEATURES[*]:-<none>})"

COMBOS=()
if (( n == 0 )); then
  # no features at all: the only two meaningful invocations
  COMBOS+=("default:")
  COMBOS+=("no-default:")
else
  COMBOS+=("default:")
  total=$(( 1 << n ))
  for (( mask = 0; mask < total; ++mask )); do
    combo=""
    for (( i = 0; i < n; ++i )); do
      if (( mask & (1 << i) )); then
        combo+="${FEATURES[$i]},"
      fi
    done
    COMBOS+=("no-default:${combo%,}")
  done
fi

rc=0
for entry in "${COMBOS[@]}"; do
  kind="${entry%%:*}"
  feats="${entry#*:}"
  if [[ "$kind" == "default" ]]; then
    args=()
    label="default features"
  else
    args=(--no-default-features)
    if [[ -n "$feats" ]]; then
      args+=(--features "$feats")
      label="--no-default-features --features $feats"
    else
      label="--no-default-features (empty feature set)"
    fi
  fi

  echo
  echo "==================================================================="
  echo "== $label"
  echo "==================================================================="

  echo "-- cargo check"
  if ! cargo check "${CARGO_FLAGS[@]}" --all-targets "${args[@]}" 2>&1 | tail -3; then
    echo "FAIL: cargo check ($label)"; rc=1; continue
  fi

  echo "-- cargo build --release (produces the cdylib under test)"
  if ! cargo build "${CARGO_FLAGS[@]}" --release "${args[@]}" 2>&1 | tail -3; then
    echo "FAIL: cargo build ($label)"; rc=1; continue
  fi

  echo "-- symbol parity"
  if ! ./check_symbols.sh; then
    echo "FAIL: symbol parity ($label)"; rc=1; continue
  fi

  # pin the .so under test so the choice is never ambiguous
  export HARVEST_RUST_SO="$here/target/release/libsh_geti_lib.so"

  echo "-- cargo test --release"
  if ! timeout 570 cargo test "${CARGO_FLAGS[@]}" --release "${args[@]}" \
         -- --test-threads=1 2>&1 | grep -E "^test |test result|^error|FAILED|panicked" ; then
    echo "FAIL: cargo test ($label)"; rc=1; continue
  fi
  # re-run capturing the summary lines so a failure is detected reliably
  if ! timeout 570 cargo test "${CARGO_FLAGS[@]}" --release "${args[@]}" \
         -- --test-threads=1 >"${TMPDIR:-/tmp}/test_out.txt" 2>&1; then
    echo "FAIL: cargo test ($label)"
    grep -E "FAILED|panicked|test result" "${TMPDIR:-/tmp}/test_out.txt" | head -40
    rc=1; continue
  fi
  grep -E "^test result" "${TMPDIR:-/tmp}/test_out.txt"
done

echo
if (( rc == 0 )); then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $rc
