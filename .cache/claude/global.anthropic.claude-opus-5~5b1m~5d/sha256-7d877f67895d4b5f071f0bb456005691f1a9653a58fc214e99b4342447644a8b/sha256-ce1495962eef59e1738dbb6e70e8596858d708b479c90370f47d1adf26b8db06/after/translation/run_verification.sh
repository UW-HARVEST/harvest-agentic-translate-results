#!/usr/bin/env bash
# Phase D driver: build the C .so, then run the whole differential suite for
# EVERY feature combination x EVERY cargo profile, and diff the symbol tables.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
export CARGO_NET_OFFLINE=true
FAIL=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

# --------------------------------------------------------------- 1. build C
say "Building the C shared library"
( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C .so: $C_SO"

# ----------------------------------------------- 2. enumerate feature combos
# Cargo features are read straight out of Cargo.toml (no guessing).
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)
echo "declared features: ${FEATURES[*]:-<none>}"

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table at all -> the crate has exactly ONE configuration.
  COMBOS+=("default")
else
  COMBOS+=("default" "no-default")
  n=${#FEATURES[@]}
  for ((m = 1; m < (1 << n); m++)); do
    set=""
    for ((i = 0; i < n; i++)); do
      if (((m >> i) & 1)); then set="${set:+$set,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("no-default:$set")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------- 3. run the suite for every combo
for profile in debug release; do
  PROF_FLAG=""
  [ "$profile" = release ] && PROF_FLAG="--release"
  for combo in "${COMBOS[@]}"; do
    case "$combo" in
      default)      FLAGS=() ;;
      no-default)   FLAGS=(--no-default-features) ;;
      no-default:*) FLAGS=(--no-default-features --features "${combo#no-default:}") ;;
    esac

    say "cargo check  [$profile] [$combo]"
    cargo check $PROF_FLAG "${FLAGS[@]}" --all-targets 2>&1 | tail -3
    [ "${PIPESTATUS[0]}" -ne 0 ] && { echo "CHECK FAILED"; FAIL=1; continue; }

    say "symbol diff  [$profile] [$combo]"
    cargo build $PROF_FLAG "${FLAGS[@]}" >/dev/null 2>&1
    R_SO="target/$profile/libreverse_collide_lib.so"
    nm -D --defined-only --format=posix "$C_SO" | awk '$2=="T"{print $1}' | sort -u > .c_syms
    nm -D --defined-only --format=posix "$R_SO" | awk '$2=="T"{print $1}' | sort -u > .r_syms
    MISSING="$(comm -23 .c_syms .r_syms)"
    if [ -n "$MISSING" ]; then
      echo "MISSING FROM RUST .so:"; echo "$MISSING"; FAIL=1
    else
      echo "OK: all $(wc -l < .c_syms) C symbols exported by the Rust .so"
    fi

    say "cargo test   [$profile] [$combo]"
    timeout 600 cargo test $PROF_FLAG "${FLAGS[@]}" 2>&1 \
      | grep -E "^test result|^running|FAILED|panicked|^error" | sed 's/^/    /'
    [ "${PIPESTATUS[0]}" -ne 0 ] && { echo "TESTS FAILED"; FAIL=1; }
  done
done

rm -f .c_syms .r_syms
say "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
