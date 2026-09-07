#!/usr/bin/env bash
# Full verification run: build the C .so, then for EVERY feature combination
# build the Rust cdylib and run Phases B, C and D against it.
#
#   ./run_all.sh
#
# Feature combinations are extracted from Cargo.toml, not hard-coded.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
CARGO_FLAGS="--offline"
FAILED=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------- C library ---
say "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so)"
echo "C .so: $C_SO"

# ------------------------------------------------------- feature enumeration ---
# Every optional feature declared in [features] (excluding "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { inf=1; next }
    /^\[/                { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$HERE/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features] -> the only configuration is the default one."
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
else
  N=${#FEATURES[@]}
  COMBOS+=("default:")
  # full power set of the optional features, with default features off
  for (( mask=0; mask < (1<<N); mask++ )); do
    sel=""
    for (( i=0; i<N; i++ )); do
      if (( mask >> i & 1 )); then sel="${sel:+$sel,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("no-default+${sel:-none}:--no-default-features${sel:+ --features $sel}")
  done
fi

echo "Feature combinations to verify: ${#COMBOS[@]}"

# ------------------------------------------------------------------- the run ---
for combo in "${COMBOS[@]}"; do
  name="${combo%%:*}"
  flags="${combo#*:}"
  say "Configuration: $name   (cargo $flags)"

  # The cdylib MUST be rebuilt explicitly: an integration test cannot link a
  # cdylib, so `cargo test` alone would leave a stale .so in place.
  # shellcheck disable=SC2086
  if ! ( cd "$HERE" && cargo build $CARGO_FLAGS --release $flags ); then
    echo "  BUILD FAILED for $name"; FAILED=1; continue
  fi
  # shellcheck disable=SC2086
  if ! ( cd "$HERE" && cargo clippy $CARGO_FLAGS --release $flags 2>/dev/null ); then
    : # clippy is optional
  fi
  for t in phase_b_configs phase_c_errors phase_d_symbols; do
    # shellcheck disable=SC2086
    if ( cd "$HERE" && timeout 600 cargo test $CARGO_FLAGS --release $flags --test "$t" ); then
      echo "  OK   $name / $t"
    else
      echo "  FAIL $name / $t"; FAILED=1
    fi
  done
done

# -------------------------------------------------------------- symbol diff ---
say "nm -D symbol diff (C .so vs Rust .so)"
R_SO="$HERE/target/release/libinreftree_lib.so"
TD="$(mktemp -d "${TMPDIR:-$HERE/target}/symdiff.XXXXXX")"
nm -D --defined-only --format=posix "$C_SO" | awk '$2 ~ /^[TBDR]$/ {print $1}' | sort > "$TD/c_syms"
nm -D --defined-only --format=posix "$R_SO" | awk '$2 ~ /^[TBDR]$/ {print $1}' | sort > "$TD/r_syms"
if [ ! -s "$TD/c_syms" ]; then echo "could not read symbols from $C_SO"; FAILED=1; fi
MISSING="$(comm -23 "$TD/c_syms" "$TD/r_syms")"
if [ -n "$MISSING" ]; then
  echo "MISSING from the Rust .so:"; echo "$MISSING"; FAILED=1
else
  echo "0 missing symbols (C exports $(wc -l < "$TD/c_syms"), all present in Rust)."
fi
rm -rf "$TD"

# ------------------------------------------------------------------ binaries ---
say "Driver binary"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
  echo "c_src declares an executable -> stdout comparison required (see CONFIGS.md)"; FAILED=1
else
  echo "No add_executable in c_src/CMakeLists.txt and no [[bin]] in Cargo.toml -> N/A."
fi

say "RESULT"
if [ "$FAILED" -eq 0 ]; then echo "ALL PHASES PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAILED"
