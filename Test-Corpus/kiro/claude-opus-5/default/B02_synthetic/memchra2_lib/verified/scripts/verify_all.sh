#!/usr/bin/env bash
# Phase D driver: builds the C .so, enumerates every Cargo feature combination,
# and for each one asserts nm -D symbol parity and runs Phases B + C.
#
# Usage: translation/scripts/verify_all.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
C_BUILD="$ROOT/c_src/build"
FAILED=0

note() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------- C shared lib
note "Building the C shared library"
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { fail "C build"; exit 1; }

C_SO="$(find "$C_BUILD" -maxdepth 1 -name 'lib*.so' | sort | tail -n1)"
[ -n "$C_SO" ] || { fail "no C .so produced"; exit 1; }
echo "C  .so: $C_SO"

# Defined GLOBAL symbols only. Drop the linker/ABI-synthesised entries that are
# not part of either library's API surface.
dynsyms() {
  nm -D --defined-only "$1" \
    | awk '$2 == "T" || $2 == "D" || $2 == "B" { print $3 }' \
    | grep -Ev '^(_init|_fini|__bss_start|_edata|_end|_IO_stdin_used)$' \
    | sort -u
}

# --------------------------------------------------- enumerate feature combos
# Read the [features] table from Cargo.toml (excluding the `default` key).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE_DIR/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "No [features] declared in Cargo.toml -> single (default) configuration."
  COMBOS+=("")                      # default build
else
  echo "Declared features: ${FEATURES[*]}"
  COMBOS+=("")                                                  # default
  COMBOS+=("--no-default-features")                             # nothing
  n=${#FEATURES[@]}
  for (( mask=1; mask < (1<<n); mask++ )); do                   # powerset
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

# ------------------------------------------------------------- per-combo gate
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  note "Configuration: $label"

  # shellcheck disable=SC2086
  ( cd "$CRATE_DIR" && timeout 600 cargo build --release $combo ) \
    >/tmp/vb.log 2>&1 || { fail "cargo build [$label]"; tail -20 /tmp/vb.log; continue; }

  RUST_SO="$CRATE_DIR/target/release/libmemchra2_lib.so"
  [ -f "$RUST_SO" ] || { fail "Rust .so missing [$label]"; continue; }

  diff_out="$(comm -23 <(dynsyms "$C_SO") <(dynsyms "$RUST_SO"))"
  if [ -n "$diff_out" ]; then
    fail "symbols exported by C but MISSING from Rust [$label]:"
    echo "$diff_out"
  else
    echo "symbol parity OK: $(dynsyms "$C_SO" | wc -l) C symbol(s), 0 missing from Rust"
  fi

  extra="$(comm -13 <(dynsyms "$C_SO") <(dynsyms "$RUST_SO"))"
  [ -n "$extra" ] && echo "note: Rust-only symbols (allowed): $(echo "$extra" | tr '\n' ' ')"

  # shellcheck disable=SC2086
  ( cd "$CRATE_DIR" && timeout 600 cargo test --release $combo ) \
    >/tmp/vt.log 2>&1 || { fail "cargo test [$label]"; tail -40 /tmp/vt.log; continue; }
  grep -E '^test result:' /tmp/vt.log
done

note "Summary"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$FAILED"
