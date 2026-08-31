#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination, then `cargo check` and
# `cargo test` each one against the C shared library.
#
# Usage: ./verify_all.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
TIMEOUT=600
status=0

echo "=== Build-time configuration discovery ==="

# Features declared in translation/Cargo.toml.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /=/   { split($0, a, "="); gsub(/[ \t"]/, "", a[1]); if (a[1] != "default") print a[1] }
  ' "$CRATE/Cargo.toml"
)
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo features declared: (none)"
else
  echo "Cargo features declared: ${FEATURES[*]}"
fi

# Build-time knobs on the C side.
echo -n "CMake options/definitions: "
if grep -Eq '^[[:space:]]*(option|add_definitions|target_compile_definitions|set\(CMAKE_C_FLAGS)' \
     "$ROOT/c_src/CMakeLists.txt"; then
  grep -En '^[[:space:]]*(option|add_definitions|target_compile_definitions|set\(CMAKE_C_FLAGS)' \
    "$ROOT/c_src/CMakeLists.txt"
else
  echo "(none)"
fi
echo -n "Conditional compilation in C sources: "
if grep -Eq '#[[:space:]]*(if|ifdef|ifndef)' "$ROOT/c_src/src/"*.c; then
  grep -En '#[[:space:]]*(if|ifdef|ifndef)' "$ROOT/c_src/src/"*.c
else
  echo "(none)"
fi

# Powerset of the declared features.
COMBOS=("")
for f in "${FEATURES[@]}"; do
  for existing in "${COMBOS[@]}"; do
    COMBOS+=("${existing:+$existing,}$f")
  done
done
echo "Feature combinations to verify: ${#COMBOS[@]}"

echo
echo "=== Building C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
  && timeout $TIMEOUT cmake --build . > /dev/null ) \
  || { echo "FAIL: C build"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
echo "built $C_SO"

syms() { nm -D --defined-only --format=posix "$1" | awk '$2 ~ /^[TDBRWViGS]$/ {print $1}' | sort -u; }

for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  echo
  echo "=== Combination: $label ==="
  args=(--no-default-features)
  [ -n "$combo" ] && args+=(--features "$combo")

  ( cd "$CRATE" && timeout $TIMEOUT cargo check --all-targets "${args[@]}" ) > /tmp/check.log 2>&1 \
    || { echo "FAIL: cargo check ($label)"; tail -30 /tmp/check.log; status=1; continue; }
  echo "cargo check: ok"

  ( cd "$CRATE" && timeout $TIMEOUT cargo build --release "${args[@]}" ) > /tmp/build.log 2>&1 \
    || { echo "FAIL: cargo build ($label)"; tail -30 /tmp/build.log; status=1; continue; }
  RUST_SO="$CRATE/target/release/libdriver.so"
  echo "cargo build: ok ($RUST_SO)"

  missing="$(comm -23 <(syms "$C_SO") <(syms "$RUST_SO"))"
  if [ -n "$missing" ]; then
    echo "FAIL: symbols exported by the C .so but missing from the Rust .so ($label):"
    echo "$missing"
    status=1
  else
    echo "symbol parity: ok ($(syms "$C_SO" | tr '\n' ' '))"
  fi

  ( cd "$CRATE" && timeout $TIMEOUT cargo test --release "${args[@]}" ) > /tmp/test.log 2>&1
  if [ $? -ne 0 ]; then
    echo "FAIL: cargo test ($label)"
    grep -E 'panicked|mismatch|test result|^---' /tmp/test.log | head -40
    status=1
  else
    echo "cargo test: ok"
    grep -E 'test result' /tmp/test.log | sed 's/^/  /'
  fi
done

echo
if [ $status -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $status
