#!/usr/bin/env bash
# Full verification sweep.
#
#  * builds the C shared library exactly as c_src/CMakeLists.txt specifies
#    (this is the CONFORMANCE TARGET);
#  * additionally builds it with -O2, purely as a diagnostic;
#  * builds the Rust cdylib for every profile x feature combination;
#  * checks nm -D symbol parity;
#  * runs the differential suite for each combination.
#
# NOTE on the -O2 build: the C source's NaN results are not stable across GCC
# optimisation levels (see the "C self-consistency" section below and NOTES.md).
# The -O0 build that CMakeLists.txt produces is therefore the only meaningful
# bit-exactness target, and the -O2 run is reported for information only.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
OUT="$CRATE/target/verify"
OFFLINE="${OFFLINE:---offline}"
mkdir -p "$OUT"
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------- C libraries
step "Building C shared library exactly as CMakeLists.txt specifies (target)"
cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build "$ROOT/c_src/build" >/dev/null || fail "C default build"
C_TARGET="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "  -> $C_TARGET"

step "Building C shared library with -O2 (out of tree; c_src is never modified)"
cmake -S "$ROOT/c_src" -B "$OUT/c_o2" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      -DCMAKE_BUILD_TYPE=Release >/dev/null \
  && cmake --build "$OUT/c_o2" >/dev/null || fail "C -O2 build"
C_O2="$(ls "$OUT"/c_o2/lib*.so | head -1)"
echo "  -> $C_O2"

# --------------------------------------------------- feature combinations
# The crate declares no [features]; enumerate anyway so that any feature added
# later is picked up automatically instead of silently going untested.
FEATS="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/[ \t]/,"",a[1]); if(a[1]!="default") print a[1]}' "$CRATE/Cargo.toml")"
echo
echo "declared features: [${FEATS:-none}]"

COMBOS=("default::" "nodefault:--no-default-features:" "allfeatures:--all-features:")
if [ -n "$FEATS" ]; then
  for f in $FEATS; do
    COMBOS+=("only-$f:--no-default-features --features $f:")
  done
fi

# ------------------------------------------------------------------ the sweep
for profile in debug release; do
  PFLAG=""; [ "$profile" = release ] && PFLAG="--release"
  for entry in "${COMBOS[@]}"; do
    name="${entry%%:*}"; rest="${entry#*:}"; fflags="${rest%%:*}"
    step "profile=$profile combo=$name flags='$fflags'"

    # cargo test does NOT build crate-type=cdylib artifacts, so build first.
    ( cd "$CRATE" && cargo build $OFFLINE $PFLAG $fflags ) >/dev/null 2>&1 \
      || { fail "cargo build ($profile/$name)"; continue; }

    RUST_SO="$CRATE/target/$profile/libhsl_to_rgb_lib.so"
    [ -f "$RUST_SO" ] || { fail "missing cdylib $RUST_SO"; continue; }

    # ---- symbol parity against the C .so
    csyms="$(nm -D --defined-only "$C_TARGET" | awk '{print $NF}' | sort -u)"
    rsyms="$(nm -D --defined-only "$RUST_SO"  | awk '{print $NF}' | sort -u)"
    missing="$(comm -23 <(echo "$csyms") <(echo "$rsyms"))"
    if [ -n "$missing" ]; then
      fail "symbols missing from Rust .so ($profile/$name):"; echo "$missing"
    else
      echo "  symbol parity: OK ($(echo "$csyms" | wc -l) C symbol(s), 0 missing)"
    fi

    # ---- CONFORMANCE: differential suite vs the CMakeLists build (must pass)
    log="$OUT/test-$profile-$name.log"
    ( cd "$CRATE" && HSL_C_SO="$C_TARGET" HSL_RUST_SO="$RUST_SO" \
        timeout 600 cargo test $OFFLINE $PFLAG $fflags -- --test-threads=4 ) \
      > "$log" 2>&1 \
      || { fail "differential suite ($profile/$name) vs the CMakeLists C build"
           grep -E '^(test .*FAILED|thread .*panicked|assertion)' "$log" | head -20; }
    grep -h '^test result:' "$log" | sed 's/^/    /'

    # ---- DIAGNOSTIC ONLY: same suite vs the -O2 C build
    log2="$OUT/test-$profile-$name-O2.log"
    ( cd "$CRATE" && HSL_C_SO="$C_O2" HSL_RUST_SO="$RUST_SO" \
        timeout 600 cargo test $OFFLINE $PFLAG $fflags -- --test-threads=4 ) \
      > "$log2" 2>&1
    if grep -q '^test result: FAILED' "$log2"; then
      echo "    [diagnostic] vs -O2 C build: mismatches (expected -- the C itself"
      echo "                 is not opt-level-stable for NaN payloads; see NOTES.md)"
    else
      echo "    [diagnostic] vs -O2 C build: also matches"
    fi
  done
done

# -------------------------------------------------- C self-consistency check
step "C self-consistency: does the C agree with ITSELF across -O0 and -O2?"
clog="$OUT/c-selfcheck.log"
( cd "$CRATE" && HSL_C_SO="$C_TARGET" HSL_C_SO_ALT="$C_O2" \
    timeout 600 cargo test $OFFLINE --release --test phase_d_c_optlevel ) > "$clog" 2>&1
if grep -q '^test result: FAILED' "$clog"; then
  echo "  NO -- the two C builds disagree with each other. Bit-exactness with both"
  echo "  is impossible, so the CMakeLists (-O0) build is the conformance target."
  grep -m3 'src=' "$clog" | sed 's/^/  /'
else
  echo "  YES -- the two C builds agree; the Rust matches both."
fi

step "SUMMARY"
if [ "$FAILED" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "THERE WERE FAILURES"; fi
exit "$FAILED"
