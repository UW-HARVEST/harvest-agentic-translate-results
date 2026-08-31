#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every valid
# build-time configuration.
#
#   1. enumerate feature combinations declared in translation/Cargo.toml
#   2. cargo check each combination
#   3. build the C shared library
#   4. cargo test each combination (differential tests via libloading)
#   5. compare exported dynamic symbols, C .so vs Rust .so
#
# Usage: ./verify.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
C_DIR="$ROOT/c_src"
RS_DIR="$ROOT/translation"
TIMEOUT=600
fail=0

note() { printf '\n=== %s ===\n' "$*"; }
bad()  { printf 'FAIL: %s\n' "$*"; fail=1; }

# --- 1. enumerate feature combinations -------------------------------------
# Read the [features] table from Cargo.toml, ignoring the implicit "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "=");
      gsub(/[[:space:]]/, "", kv[1]);
      if (kv[1] != "default") print kv[1];
    }
  ' "$RS_DIR/Cargo.toml"
)

# Every subset of the declared features, expressed as a comma-separated list.
# An empty list means --no-default-features with nothing enabled.
COMBOS=()
n=${#FEATURES[@]}
for (( mask = 0; mask < (1 << n); mask++ )); do
  combo=()
  for (( i = 0; i < n; i++ )); do
    (( mask & (1 << i) )) && combo+=("${FEATURES[i]}")
  done
  COMBOS+=("$(IFS=,; echo "${combo[*]}")")
done
# `default` is also a valid configuration in its own right.
COMBOS+=("<default>")

note "feature combinations (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do echo "  ${c:-<none>}"; done

# Translate a combo label into cargo flags.
cargo_flags() {
  if [[ "$1" == "<default>" ]]; then
    echo ""
  elif [[ -z "$1" ]]; then
    echo "--no-default-features"
  else
    echo "--no-default-features --features $1"
  fi
}

# --- 2. cargo check every combination --------------------------------------
note "cargo check"
for c in "${COMBOS[@]}"; do
  flags=$(cargo_flags "$c")
  if timeout "$TIMEOUT" cargo check --manifest-path "$RS_DIR/Cargo.toml" \
       --all-targets $flags >/dev/null 2>&1; then
    echo "  ok    ${c:-<none>}"
  else
    bad "cargo check ${c:-<none>}"
    timeout "$TIMEOUT" cargo check --manifest-path "$RS_DIR/Cargo.toml" \
      --all-targets $flags 2>&1 | tail -20
  fi
done

# --- 3. build the C shared library ----------------------------------------
note "build C shared library"
mkdir -p "$C_DIR/build"
if ! ( cd "$C_DIR/build" \
       && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
       && timeout "$TIMEOUT" cmake --build . >/dev/null ); then
  bad "C build"
  exit 1
fi
C_SO="$C_DIR/build/libdriver.so"
echo "  ok    $C_SO"

# --- 4. differential tests per combination --------------------------------
note "cargo test (differential, C vs Rust through libloading)"
for c in "${COMBOS[@]}"; do
  flags=$(cargo_flags "$c")
  if timeout "$TIMEOUT" cargo test --manifest-path "$RS_DIR/Cargo.toml" \
       $flags >/tmp/driver-test-$$.log 2>&1; then
    echo "  ok    ${c:-<none>}"
  else
    bad "cargo test ${c:-<none>}"
    tail -40 /tmp/driver-test-$$.log
  fi
done
rm -f /tmp/driver-test-$$.log

# --- 5. exported symbol parity --------------------------------------------
note "exported dynamic symbol parity"
dynsyms() { nm -D --defined-only "$1" 2>/dev/null | awk '{print $NF}' | sort -u; }

for c in "${COMBOS[@]}"; do
  flags=$(cargo_flags "$c")
  if ! timeout "$TIMEOUT" cargo build --manifest-path "$RS_DIR/Cargo.toml" \
         --release --lib $flags >/dev/null 2>&1; then
    bad "cargo build --release ${c:-<none>}"
    continue
  fi
  RS_SO="$RS_DIR/target/release/libdriver.so"
  missing=$(comm -23 <(dynsyms "$C_SO") <(dynsyms "$RS_SO"))
  if [[ -n "$missing" ]]; then
    bad "Rust .so is missing symbols exported by the C .so (${c:-<none>}):"
    echo "$missing" | sed 's/^/    /'
  else
    echo "  ok    ${c:-<none>} (all C symbols present: $(dynsyms "$C_SO" | tr '\n' ' '))"
  fi
done

note "result"
if (( fail )); then
  echo "FAILED"
  exit 1
fi
echo "PASSED"
