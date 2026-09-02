#!/usr/bin/env bash
# Full verification sweep: builds both libraries, diffs their exported symbol
# tables, and runs the differential test suite across every cargo feature
# combination and both profiles.
#
# Usage:  ./verify.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
C_SRC="$ROOT/c_src"
C_SO="$C_SRC/build/libhello.so"
FAILURES=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL\033[0m %s\n' "$*"; FAILURES=$((FAILURES + 1)); }
pass() { printf '\033[32mok\033[0m   %s\n' "$*"; }

# --------------------------------------------------------------- build the C
say "Building the C shared library"
mkdir -p "$C_SRC/build"
( cd "$C_SRC/build" \
    && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && timeout 600 cmake --build . >/dev/null ) \
  && pass "libhello.so built" || fail "C build failed"
[[ -f "$C_SO" ]] || { fail "missing $C_SO"; exit 1; }

# ------------------------------------------- enumerate feature combinations
# Every subset of the optional features declared in Cargo.toml, plus the
# default build and the no-default-features build.
say "Enumerating cargo feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' "$HERE/Cargo.toml"
)
echo "optional features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=()                     # each entry is a set of cargo flags
COMBOS+=("")                                        # default features
COMBOS+=("--no-default-features")                   # nothing enabled
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    sel=()
    for (( i = 0; i < n; i++ )); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    COMBOS+=("--features $(IFS=,; echo "${sel[*]}")")
  done
fi
if (( n > 0 )); then
  COMBOS+=("--all-features")
fi
echo "combinations to verify: ${#COMBOS[@]}"

# ------------------------------------------------------- sweep every combo
for profile in debug release; do
  PROF_FLAG=""
  [[ $profile == release ]] && PROF_FLAG="--release"
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-default}]"

    say "cargo check — $label"
    if timeout 600 cargo check $PROF_FLAG $combo --all-targets >/tmp/hv_check.log 2>&1; then
      pass "check: $label"
    else
      fail "check: $label"; tail -25 /tmp/hv_check.log
      continue
    fi

    say "cargo build — $label"
    if timeout 600 cargo build $PROF_FLAG $combo >/tmp/hv_build.log 2>&1; then
      pass "build: $label"
    else
      fail "build: $label"; tail -25 /tmp/hv_build.log
      continue
    fi

    RUST_SO="$HERE/target/$profile/libhello.so"
    if [[ ! -f $RUST_SO ]]; then
      fail "no Rust .so at $RUST_SO for $label"
      continue
    fi

    # ---- symbol parity for this exact build
    say "symbol parity — $label"
    diff <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sed 's/@.*//' | sort -u) \
         <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u) \
         >/tmp/hv_syms.diff
    if grep -q '^<' /tmp/hv_syms.diff; then
      fail "symbols missing from the Rust .so ($label):"
      grep '^<' /tmp/hv_syms.diff
    else
      pass "every C export present in the Rust .so ($label)"
    fi

    # ---- differential suite
    say "cargo test — $label"
    if HELLO_C_SO="$C_SO" HELLO_RUST_SO="$RUST_SO" \
         timeout 600 cargo test $PROF_FLAG $combo -- --test-threads=1 \
         >/tmp/hv_test.log 2>&1; then
      pass "tests: $label"
      grep -E '^test result:' /tmp/hv_test.log | sed 's/^/       /'
    else
      fail "tests: $label"
      tail -60 /tmp/hv_test.log
    fi
  done
done

# ------------------------------------------------------------------ summary
say "Summary"
echo "C   exports: $(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u | tr '\n' ' ')"
echo "Rust exports: $(nm -D --defined-only "$HERE/target/release/libhello.so" | awk '{print $NF}' | sed 's/@.*//' | sort -u | tr '\n' ' ')"
if (( FAILURES == 0 )); then
  printf '\n\033[32mALL CHECKS PASSED\033[0m\n'
  exit 0
fi
printf '\n\033[31m%d CHECK(S) FAILED\033[0m\n' "$FAILURES"
exit 1
