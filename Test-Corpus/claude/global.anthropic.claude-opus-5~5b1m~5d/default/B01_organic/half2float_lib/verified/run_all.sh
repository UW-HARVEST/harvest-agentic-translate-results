#!/usr/bin/env bash
# Full verification sweep: builds both libraries, checks symbol parity, and runs
# the differential suite under every configuration (profile x feature combo).
#
# Usage: ./run_all.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
CARGO_FLAGS="--offline"   # local registry cache; drop if network is available
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- build C -----
step "Building C shared library"
cmake -S "$ROOT/c_src" -B "$CBUILD" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null || exit 1
cmake --build "$CBUILD" >/dev/null || exit 1
C_SO="$(find "$CBUILD" -maxdepth 1 -name 'lib*.so' | head -1)"
echo "C .so:  $C_SO"

# ------------------------------------------------------------- build Rust -----
step "Building Rust cdylib (release + debug)"
cd "$CRATE" || exit 1
cargo build $CARGO_FLAGS --release -q || exit 1
cargo build $CARGO_FLAGS -q         || exit 1

# --------------------------------------------------- feature combinations -----
# Enumerate features straight out of Cargo.toml rather than hardcoding them.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[a-zA-Z0-9_-]+ *=/{print $1}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> single feature combination"
  COMBOS=("default" "no-default-features")
else
  COMBOS=("default" "no-default-features")
  for f in $FEATURES; do COMBOS+=("$f"); done
fi

# --------------------------------------------------------- symbol parity ------
step "Phase D: symbol parity (nm -D)"
for PROFILE in release debug; do
  R_SO="$CRATE/target/$PROFILE/libhalf2float_lib.so"
  c_syms=$(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u)
  r_syms=$(nm -D --defined-only "$R_SO"  | awk '{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  echo "[$PROFILE] C exports: $(echo "$c_syms" | grep -c .), Rust exports: $(echo "$r_syms" | grep -c .)"
  if [ -n "$missing" ]; then
    echo "[$PROFILE] MISSING FROM RUST:"; echo "$missing"; FAIL=1
  else
    echo "[$PROFILE] symbol diff EMPTY ✅"
  fi
  # Undefined non-libc symbols in the Rust .so.
  # Anything versioned @GLIBC/@GCC is libc/runtime; _ITM_* and __gmon_start__
  # are the standard weak GCC transactional-memory / profiling stubs.
  undef=$(nm -D -u "$R_SO" | awk '{print $NF}' \
    | grep -vE '@(GLIBC|GLIBC_[0-9.]+|GCC[_.0-9]*)' \
    | grep -vE '^(_ITM_(de)?registerTMCloneTable|__gmon_start__|__cxa_[a-z_]+|_Unwind_[A-Za-z]+)$' \
    | grep -vE '^$' || true)
  if [ -n "$undef" ]; then
    echo "[$PROFILE] undefined NON-libc symbols (must be empty):"; echo "$undef"; FAIL=1
  else
    echo "[$PROFILE] 0 undefined non-libc symbols ✅"
  fi
done

# ------------------------------------------------- differential test runs -----
for PROFILE in release debug; do
  for COMBO in "${COMBOS[@]}"; do
    case "$COMBO" in
      default)             FEAT_ARG="" ;;
      no-default-features) FEAT_ARG="--no-default-features" ;;
      *)                   FEAT_ARG="--no-default-features --features $COMBO" ;;
    esac
    step "Phase B+C: profile=$PROFILE features=$COMBO"
    # Test binary always built in release for speed; HALF2FLOAT_PROFILE selects
    # WHICH cdylib artifact is dlopen'd, which is what actually varies.
    if HALF2FLOAT_PROFILE="$PROFILE" timeout 600 \
         cargo test $CARGO_FLAGS --release $FEAT_ARG 2>&1 | tail -5 | grep -q 'test result: ok'; then
      echo "PASS (profile=$PROFILE, features=$COMBO)"
    else
      echo "FAIL (profile=$PROFILE, features=$COMBO)"; FAIL=1
    fi
  done
done

step "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASS ✅"
else
  echo "FAILURES DETECTED ❌"
fi
exit "$FAIL"
