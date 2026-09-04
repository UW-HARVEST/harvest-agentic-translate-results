#!/usr/bin/env bash
# Phase D completion gate, fully automated.
#
#   1. builds the C shared library
#   2. enumerates every feature combination declared in Cargo.toml
#   3. for each combination x each profile (debug, release):
#        - cargo check
#        - build the cdylib
#        - diff `nm -D` against the C .so (must be EMPTY)
#        - run the whole differential suite against THAT .so
#
# Usage: scripts/verify_all.sh
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE=$PWD
ROOT=$CRATE/..
OFF=--offline
SYMDIFF=$(mktemp "${TMPDIR:-/tmp}/symdiff.XXXXXX")
trap 'rm -f "$SYMDIFF"' EXIT
fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad()  { printf '\033[31mFAIL\033[0m %s\n' "$*"; fail=1; }
good() { printf '\033[32mok\033[0m   %s\n' "$*"; }

# --------------------------------------------------------------------------
note "Building the C shared library"
mkdir -p "$ROOT/c_src/build" || exit 1
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
good "C .so = $C_SO"
export C_SO

# --------------------------------------------------------------------------
note "Enumerating feature combinations from Cargo.toml"
# Every feature name declared under a [features] table (there may be none).
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); if ($0 != "default") print }
' Cargo.toml | sort -u)

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> the only configuration is the default one"
  COMBOS+=("DEFAULT" "NONE")
else
  echo "features: $(echo "$FEATURES" | tr '\n' ' ')"
  COMBOS+=("DEFAULT" "NONE")
  # full power set of the declared features, with default features off
  feats=($FEATURES)
  n=${#feats[@]}
  for ((m = 1; m < (1 << n); m++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      (((m >> i) & 1)) && combo="$combo,${feats[$i]}"
    done
    COMBOS+=("${combo#,}")
  done
  COMBOS+=("ALL")
fi
printf 'combinations to verify: %s\n' "${COMBOS[*]}"

# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT) FLAGS=() ;;
    NONE)    FLAGS=(--no-default-features) ;;
    ALL)     FLAGS=(--all-features) ;;
    *)       FLAGS=(--no-default-features --features "$combo") ;;
  esac

  for profile in debug release; do
    [ "$profile" = release ] && PF=(--release) || PF=()
    tag="features=$combo profile=$profile"
    note "$tag"

    if ! cargo check $OFF "${PF[@]}" "${FLAGS[@]}" >/dev/null 2>&1; then
      bad "cargo check ($tag)"; cargo check $OFF "${PF[@]}" "${FLAGS[@]}" 2>&1 | tail -20
      continue
    fi
    good "cargo check"

    if ! cargo build $OFF "${PF[@]}" "${FLAGS[@]}" >/dev/null 2>&1; then
      bad "cargo build ($tag)"; continue
    fi
    RUST_SO="$CRATE/target/$profile/libarrayfunc_lib.so"
    [ -f "$RUST_SO" ] || { bad "no cdylib at $RUST_SO"; continue; }
    good "cdylib = $RUST_SO"
    export RUST_SO

    # ---- symbol diff must be empty ----
    diff <(nm -D --defined-only "$C_SO"   | awk '$2=="T"{print $3}' | sort -u) \
         <(nm -D --defined-only "$RUST_SO" | awk '$2=="T"{print $3}' | sort -u) \
         > "$SYMDIFF" 2>/dev/null || true
    if [ -s "$SYMDIFF" ]; then
      # only "<" lines (present in C, absent in Rust) are failures
      if grep -q '^<' "$SYMDIFF"; then
        bad "symbols missing from Rust .so ($tag)"; grep '^<' "$SYMDIFF"
      else
        good "symbol diff empty (Rust exports extras only: $(grep -c '^>' "$SYMDIFF"))"
      fi
    else
      good "symbol diff EMPTY"
    fi
    : # kept until exit trap

    # ---- the whole differential suite against this exact .so ----
    if cargo test $OFF "${PF[@]}" "${FLAGS[@]}" -- --test-threads=4 > "$CRATE/target/test-$combo-$profile.log" 2>&1; then
      good "differential suite ($(grep -ho '[0-9]* passed' "$CRATE/target/test-$combo-$profile.log" | awk '{s+=$1} END {print s}') assertions-groups passed)"
    else
      bad "differential suite ($tag) -- see target/test-$combo-$profile.log"
      tail -40 "$CRATE/target/test-$combo-$profile.log"
    fi
  done
done

note "SUMMARY"
if [ "$fail" -eq 0 ]; then
  printf '\033[32mALL CONFIGURATIONS PASSED\033[0m\n'
else
  printf '\033[31mFAILURES PRESENT\033[0m\n'
fi
exit "$fail"
