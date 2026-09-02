#!/usr/bin/env bash
# Full verification driver: builds both shared objects, runs the differential
# suite under EVERY cargo feature combination, and re-checks symbol parity.
#
# Usage:  ./scripts/verify_all.sh [--with-mutations]
set -euo pipefail

cd "$(dirname "$0")/.."
CRATE="$PWD"
ROOT="$(cd .. && pwd)"

echo "==> building the C shared library"
(
  cd "$ROOT/c_src"
  mkdir -p build
  cd build
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
)
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)
echo "    C  .so: $C_SO"

echo "==> enumerating cargo feature combinations from Cargo.toml"
# Extract feature names from the [features] table, if any.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

# Build the list of invocations to test.
COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "    no [features] table -> the default (empty) feature set is the only configuration"
  COMBOS+=("--no-default-features")
  COMBOS+=("")            # default
  COMBOS+=("--all-features")
else
  echo "    features: ${FEATURES[*]}"
  COMBOS+=("--no-default-features")
  COMBOS+=("")
  COMBOS+=("--all-features")
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel+=("${FEATURES[i]}"); fi
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-(default)}"
  echo
  echo "==> cargo check   $label"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check --release $combo --quiet; then
    echo "    CHECK FAILED: $label"
    fail=1
    continue
  fi
  echo "==> cargo build   $label"
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $combo --quiet
  echo "==> nm -D parity  $label"
  c_syms=$(nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort)
  r_syms=$(nm -D --defined-only "$CRATE/target/release/libtfm_lib.so" | awk '$2=="T"{print $3}' | sort)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [ -n "$missing" ]; then
    echo "    MISSING SYMBOLS in Rust .so: $missing"
    fail=1
  else
    echo "    symbol diff empty ($(echo "$c_syms" | wc -l) C symbol(s))"
  fi
  echo "==> cargo test    $label"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test --release $combo -- --test-threads="$(nproc)"; then
    echo "    TESTS FAILED: $label"
    fail=1
  fi
done

if [ "${1:-}" = "--with-mutations" ]; then
  echo
  echo "==> mutation check (negative control)"
  timeout 900 python3 scripts/mutation_check.py || fail=1
fi

echo
if [ "$fail" -ne 0 ]; then
  echo "VERIFICATION FAILED"
  exit 1
fi
echo "VERIFICATION COMPLETE — all feature combinations pass, symbol diff empty"
