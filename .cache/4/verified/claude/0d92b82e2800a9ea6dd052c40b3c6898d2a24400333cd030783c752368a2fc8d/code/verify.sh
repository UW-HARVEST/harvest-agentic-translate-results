#!/usr/bin/env bash
# Full differential-verification run: builds the C and Rust shared objects,
# diffs their exported symbols, and runs the Phase B / Phase C test suites for
# every feature combination and both cargo profiles.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
FAIL=0
LOGDIR="${TMPDIR:-$ROOT/verify-logs}"
mkdir -p "$LOGDIR" 2>/dev/null || LOGDIR="$ROOT/verify-logs"
mkdir -p "$LOGDIR"

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   [ok] %s\n' "$*"; }
bad()  { printf '   [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "1. Build the C shared library"
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . ) >$LOGDIR/cbuild.log 2>&1 \
  && ok "c_src/build/libtranslated_rust.so" || { bad "C build (see $LOGDIR/cbuild.log)"; tail -20 $LOGDIR/cbuild.log; }

# ---------------------------------------------------------------------------
step "2. Enumerate feature combinations from Cargo.toml"
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,"");print}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "   Cargo.toml declares no [features]; the only build configuration is the default."
  COMBOS=("")           # empty string == --no-default-features with nothing extra
else
  # power set of the declared features
  mapfile -t FLIST <<<"$FEATURES"
  n=${#FLIST[@]}
  COMBOS=()
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FLIST[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
printf '   combos: %s\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
step "3. cargo check for every feature combination"
for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then args=(--no-default-features); else args=(--no-default-features --features "$c"); fi
  if timeout 600 cargo check "${args[@]}" >$LOGDIR/check.log 2>&1; then
    ok "cargo check ${args[*]}"
  else
    bad "cargo check ${args[*]}"; tail -25 $LOGDIR/check.log
  fi
done
# also the default and all-features spellings
for extra in "" "--all-features"; do
  if timeout 600 cargo check $extra >$LOGDIR/check.log 2>&1; then ok "cargo check $extra"; else bad "cargo check $extra"; tail -25 $LOGDIR/check.log; fi
done

# ---------------------------------------------------------------------------
step "4. Symbol parity (nm -D)"
for prof in debug release; do
  if [ "$prof" = release ]; then timeout 600 cargo build --release --quiet; else timeout 600 cargo build --quiet; fi
  RS="target/$prof/librgb_to_hsv_lib.so"
  C_SO="c_src/build/libtranslated_rust.so"
  nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort > $LOGDIR/c.syms
  nm -D --defined-only "$RS"   | awk '$2=="T"{print $3}' | sort > $LOGDIR/r.syms
  MISSING=$(comm -23 $LOGDIR/c.syms $LOGDIR/r.syms)
  if [ -z "$MISSING" ]; then
    ok "$prof: all $(wc -l <$LOGDIR/c.syms) C symbol(s) exported by the Rust .so"
  else
    bad "$prof: missing from Rust .so:"; echo "$MISSING"
  fi
  UNDEF=$(nm -D --undefined-only "$RS" | awk '{print $NF}' | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^gettid$|^statx$|^__cxa_thread_atexit_impl$' || true)
  if [ -z "$UNDEF" ]; then ok "$prof: no undefined non-libc symbols"; else bad "$prof: undefined: $UNDEF"; fi
done

# ---------------------------------------------------------------------------
step "5. Differential tests (Phase B + Phase C) for every combo and profile"
for c in "${COMBOS[@]}"; do
  for prof in "" "--release"; do
    if [ -z "$c" ]; then args=(--no-default-features); else args=(--no-default-features --features "$c"); fi
    [ -n "$prof" ] && args+=("$prof")
    if timeout 600 cargo test "${args[@]}" >$LOGDIR/test.log 2>&1; then
      ok "cargo test ${args[*]}  ($(grep -c '^test .* ok$' $LOGDIR/test.log) tests)"
    else
      bad "cargo test ${args[*]}"; grep -E "^test .* FAILED|panicked at|test result" $LOGDIR/test.log | head -30
    fi
  done
done
# default / all-features spellings
for extra in "" "--all-features"; do
  if timeout 600 cargo test $extra >$LOGDIR/test.log 2>&1; then
    ok "cargo test $extra  ($(grep -c '^test .* ok$' $LOGDIR/test.log) tests)"
  else
    bad "cargo test $extra"; grep -E "^test .* FAILED|panicked at|test result" $LOGDIR/test.log | head -30
  fi
done

# ---------------------------------------------------------------------------
step "6. Same suite against optimised C builds (-O2 / -O3 -march=native)"
for opt in "-O2" "-O3 -march=native"; do
  SO="$LOGDIR/libc_$(echo "$opt" | tr -d ' -=')".so
  if cc $opt -shared -fPIC -Ic_src/include -o "$SO" c_src/src/lib.c 2>$LOGDIR/copt.log; then
    if HARVEST_C_SO="$SO" timeout 600 cargo test --no-default-features >$LOGDIR/test.log 2>&1; then
      ok "cargo test vs C built with '$opt'  ($(grep -c '^test .* ok$' $LOGDIR/test.log) tests)"
    else
      bad "cargo test vs C built with '$opt'"; grep -E "^test .* FAILED|panicked at|test result" $LOGDIR/test.log | head -30
    fi
  else
    echo "   (skipped: cc $opt unavailable)"
  fi
done

# ---------------------------------------------------------------------------
step "Result"
if [ "$FAIL" = 0 ]; then echo "   ALL CHECKS PASSED"; else echo "   FAILURES PRESENT"; fi
exit $FAIL
