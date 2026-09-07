#!/usr/bin/env bash
# Phase D driver: run the full differential suite under EVERY feature
# combination and against BOTH cargo profiles' .so artifacts, then re-check
# symbol parity. Everything is derived from Cargo.toml, nothing hard-coded.
set -uo pipefail
cd "$(dirname "$0")/.."   # translation/
ROOT="$(cd .. && pwd)"

fail=0

# ---------------------------------------------------------------------------
# 1. Enumerate the feature combinations declared in Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml)

echo "=== declared features ==="
if [ -z "$FEATURES" ]; then
  echo "(none - Cargo.toml declares no [features] section)"
else
  echo "$FEATURES"
fi

# Build the list of combos to test. With no declared features there is exactly
# one build configuration; assert that --no-default-features and --all-features
# are equivalent to it rather than assuming so.
COMBOS=("" "--no-default-features" "--all-features")
if [ -n "$FEATURES" ]; then
  # Power set of the declared features.
  feats=($FEATURES)
  n=${#feats[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${feats[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $combo")
  done
fi

# ---------------------------------------------------------------------------
# 2. Build the C reference library
# ---------------------------------------------------------------------------
echo
echo "=== building C reference .so ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
echo "  $C_SO"

# ---------------------------------------------------------------------------
# 3. For each combo x profile: build, diff symbols, run the whole suite
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    label="combo='${combo:-<default>}' profile=$profile"
    echo
    echo "=================================================================="
    echo "=== $label"
    echo "=================================================================="

    if [ "$profile" = release ]; then
      pflag="--release"; pdir=release
    else
      pflag=""; pdir=debug
    fi

    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $pflag $combo >/dev/null 2>&1; then
      echo "  BUILD FAILED"; fail=1; continue
    fi
    RUST_SO="$PWD/target/$pdir/libdriver.so"

    # --- symbol parity -----------------------------------------------------
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u))
    if [ -n "$missing" ]; then
      echo "  SYMBOL PARITY FAILED - missing from Rust .so:"; echo "$missing"; fail=1
    else
      echo "  symbol parity: OK (0 missing)"
    fi
    if nm -D --undefined-only "$RUST_SO" | grep -qv \
        -e 'GLIBC' -e 'GCC_' -e '_ITM_' -e '__gmon_start__' -e 'statx' -e 'gettid'; then
      echo "  NOTE: non-libc/runtime undefined symbols present:"
      nm -D --undefined-only "$RUST_SO" | grep -v \
        -e 'GLIBC' -e 'GCC_' -e '_ITM_' -e '__gmon_start__' -e 'statx' -e 'gettid'
    fi
    if ldd "$RUST_SO" | grep -q 'not found'; then
      echo "  UNRESOLVED DEPENDENCIES"; ldd "$RUST_SO" | grep 'not found'; fail=1
    fi

    # --- full differential suite, against THIS .so -------------------------
    # DRIVER_RUST_SO pins the artifact under test so the release .so (the
    # shipped one) is verified too, not just the debug build.
    # shellcheck disable=SC2086
    if DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$RUST_SO" \
       timeout 600 cargo test $combo -- --test-threads=1 >/tmp/dt.$$ 2>&1; then
      grep -E '^test result:' /tmp/dt.$$ | sed 's/^/  /'
    else
      echo "  TESTS FAILED"; tail -40 /tmp/dt.$$; fail=1
    fi
    rm -f /tmp/dt.$$
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS x PROFILES: PASS"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
