#!/usr/bin/env bash
# Full verification driver: every feature combination x every cargo profile.
#
# `Cargo.toml` declares `[features] default = []` and `c_src/CMakeLists.txt` has
# no build-time configuration at all (no option(), no #ifdef, no NDEBUG), so the
# feature-combination set is a single, empty combination.  It is still enumerated
# mechanically below rather than hard-coded.
set -uo pipefail
cd "$(dirname "$0")"

TMP="${TMPDIR:-/tmp}"
fail=0

# ---------------------------------------------------------------------------
# 0. build the C shared library
# ---------------------------------------------------------------------------
echo "=== building the C shared library ==="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$TMP/cmake.log" 2>&1 \
  && cmake --build . >>"$TMP/cmake.log" 2>&1 ) \
  || { echo "FAIL: C build"; tail -20 "$TMP/cmake.log"; exit 1; }
C_SO=c_src/build/libtranslated_rust.so
echo "ok: $C_SO"

# ---------------------------------------------------------------------------
# 1. enumerate the feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' Cargo.toml)
echo "=== optional features declared: [${FEATURES:-<none>}] ==="

COMBOS=("")            # the empty combination
if [ -n "$FEATURES" ]; then
  # power set of the declared features
  set -- $FEATURES
  n=$#
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((b=0; b<n; b++)); do
      if (( mask & (1<<b) )); then
        eval "f=\${$((b+1))}"
        combo="${combo:+$combo,}$f"
      fi
    done
    COMBOS+=("$combo")
  done
fi
if [ -n "$FEATURES" ]; then
  COMBOS+=("__ALL__")  # --all-features differs from the empty combination
else
  echo "note: no optional features exist, so --all-features, --no-default-features"
  echo "      and the default build are the *same* compilation; the check below"
  echo "      still runs --all-features explicitly, the test run does not repeat it."
  ALL_IS_DUP=1
fi
echo "=== ${#COMBOS[@]} feature combination(s) to test ==="

# ---------------------------------------------------------------------------
# 2. cargo check for every combination (plus --all-features, always)
# ---------------------------------------------------------------------------
CHECK_COMBOS=("${COMBOS[@]}")
[ "${ALL_IS_DUP:-0}" = 1 ] && CHECK_COMBOS+=("__ALL__")
for combo in "${CHECK_COMBOS[@]}"; do
  if [ "$combo" = "__ALL__" ]; then
    args=(--all-features)
    name="--all-features"
  elif [ -z "$combo" ]; then
    args=(--no-default-features)
    name="--no-default-features"
  else
    args=(--no-default-features --features "$combo")
    name="--no-default-features --features $combo"
  fi
  printf 'cargo check %-46s ... ' "$name"
  if timeout 600 cargo check --offline --all-targets "${args[@]}" >"$TMP/check.log" 2>&1; then
    echo ok
  else
    echo FAIL; fail=1; tail -25 "$TMP/check.log"
  fi
done

# ---------------------------------------------------------------------------
# 3. symbol parity, for every profile
# ---------------------------------------------------------------------------
for profile in debug release; do
  echo "=== $profile: build + symbol parity ==="
  if [ "$profile" = release ]; then
    timeout 600 cargo build --offline --release >"$TMP/build.log" 2>&1 || { echo FAIL; fail=1; continue; }
  else
    timeout 600 cargo build --offline >"$TMP/build.log" 2>&1 || { echo FAIL; fail=1; continue; }
  fi
  R_SO="target/$profile/libload_png_mem_lib.so"
  [ -f "$R_SO" ] || R_SO="target/$profile/deps/libload_png_mem_lib.so"
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u >"$TMP/c.syms"
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u >"$TMP/r.syms"
  missing=$(comm -23 "$TMP/c.syms" "$TMP/r.syms")
  if [ -n "$missing" ]; then
    echo "FAIL: symbols exported by the C .so but missing from $R_SO:"
    echo "$missing"; fail=1
  else
    echo "ok: all $(wc -l <"$TMP/c.syms") C symbols are exported by $R_SO"
  fi
done

# ---------------------------------------------------------------------------
# 4. the differential test suite, for every combination x profile
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "__ALL__" ]; then
    args=(--all-features); name="--all-features"
  elif [ -z "$combo" ]; then
    args=(--no-default-features); name="--no-default-features"
  else
    args=(--no-default-features --features "$combo"); name="--features $combo"
  fi
  for profile in "" "--release"; do
    printf 'cargo test %s %-34s ... ' "${profile:-(dev)}" "$name"
    if timeout 600 cargo test --offline "${args[@]}" $profile -- --test-threads=1 >"$TMP/test.log" 2>&1; then
      echo "ok ($(grep -c '^test .* ok$' "$TMP/test.log") tests)"
    else
      echo FAIL; fail=1
      grep -E 'test result|FAILED|panicked' "$TMP/test.log" | head -20
    fi
  done
done

echo
if [ "$fail" = 0 ]; then echo "ALL VERIFICATION PASSED"; else echo "VERIFICATION FAILED"; fi
exit "$fail"
