#!/usr/bin/env bash
# Differential verification driver: builds the C shared library, enumerates
# every valid Cargo feature combination, and for each one runs
#   cargo check -> cargo build --lib -> nm symbol diff -> cargo test
# in both the dev and the release profile.
#
# Usage: ./run_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
LOG_DIR="${TMPDIR:-/tmp}/difftest-logs"
mkdir -p "$LOG_DIR"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }
bad()  { printf '!!! FAIL: %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate the feature combinations declared in Cargo.toml.
# ---------------------------------------------------------------------------
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

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  note "Cargo.toml declares no [features]: the only configuration is the empty one"
  COMBOS=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
printf 'feature combinations to verify: %d\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '  - [%s]\n' "${c:-<none>}"; done

# ---------------------------------------------------------------------------
# 2. Build the C shared library (the ground truth).
# ---------------------------------------------------------------------------
note "building the C shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOG_DIR/cmake.log" 2>&1 \
  || { bad "cmake build"; tail -30 "$LOG_DIR/cmake.log"; exit 1; }

C_SO=$(ls c_src/build/*.so | head -1)
printf 'C .so: %s\n' "$C_SO"
nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TWDBRV]$/ { print $3 }' | sort -u > "$LOG_DIR/c_syms.txt"
printf 'C exports %d symbols\n' "$(wc -l < "$LOG_DIR/c_syms.txt")"

# ---------------------------------------------------------------------------
# 3. For every combination x profile: check, build, symbol-diff, test.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  featflag=(--no-default-features)
  [ -n "$combo" ] && featflag+=(--features "$combo")

  for profile in dev release; do
    label="features=[${combo:-<none>}] profile=$profile"
    profflag=()
    outdir="target/debug"
    if [ "$profile" = release ]; then
      profflag=(--release)
      outdir="target/release"
    fi
    tag="${combo:-none}-$profile"
    tag="${tag//,/_}"

    note "cargo check  ($label)"
    if ! timeout 600 cargo check "${featflag[@]}" "${profflag[@]}" --all-targets \
         > "$LOG_DIR/check-$tag.log" 2>&1; then
      bad "cargo check ($label)"; tail -40 "$LOG_DIR/check-$tag.log"; continue
    fi
    grep -E "^(warning|error)" "$LOG_DIR/check-$tag.log" | sort -u | head -10

    note "cargo build --lib  ($label)"
    if ! timeout 600 cargo build --lib "${featflag[@]}" "${profflag[@]}" \
         > "$LOG_DIR/build-$tag.log" 2>&1; then
      bad "cargo build --lib ($label)"; tail -40 "$LOG_DIR/build-$tag.log"; continue
    fi

    RUST_SO="$outdir/libpoly_ray_lib.so"
    if [ ! -f "$RUST_SO" ]; then
      bad "missing $RUST_SO ($label)"; continue
    fi

    note "nm symbol diff  ($label)"
    nm -D --defined-only "$RUST_SO" | awk '$2 ~ /^[TWDBRV]$/ { print $3 }' | sort -u \
      > "$LOG_DIR/rust_syms-$tag.txt"
    missing=$(comm -23 "$LOG_DIR/c_syms.txt" "$LOG_DIR/rust_syms-$tag.txt")
    if [ -n "$missing" ]; then
      bad "symbols missing from the Rust .so ($label):"
      printf '  %s\n' $missing
    else
      printf 'symbol diff EMPTY: all %d C symbols are exported by the Rust .so\n' \
        "$(wc -l < "$LOG_DIR/c_syms.txt")"
    fi

    note "cargo test  ($label)"
    if ! timeout 600 cargo test "${featflag[@]}" "${profflag[@]}" \
         > "$LOG_DIR/test-$tag.log" 2>&1; then
      bad "cargo test ($label)"; tail -60 "$LOG_DIR/test-$tag.log"; continue
    fi
    grep -E "^test result:" "$LOG_DIR/test-$tag.log"
    awk '/^test result:/ { p += $4; f += $6 } END { printf "TOTAL: %d passed, %d failed\n", p, f }' \
      "$LOG_DIR/test-$tag.log"
  done
done

note "SUMMARY"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES DETECTED (see above)"
fi
exit "$fail"
