#!/usr/bin/env bash
# Phase D sweep: build the C .so, then run the full differential suite for
# every Cargo feature combination and for both profiles.
#
# Usage: translation/scripts/verify_all.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(cd "$HERE/.." && pwd)"
ROOT="$(cd "$CRATE/.." && pwd)"
CARGO_FLAGS="--offline"

fail=0

echo "== building the C shared library =="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
test -f "$ROOT/c_src/build/libdriver.so" || { echo "libdriver.so missing"; exit 1; }

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,""); gsub(/ /,""); if ($0!="default" && $0!="") print}' \
    "$CRATE/Cargo.toml"
)

declare -a COMBOS_DESC=() COMBOS_FLAGS=()
COMBOS_DESC+=("default features");            COMBOS_FLAGS+=("")
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "== Cargo.toml declares no [features]; the only configuration is the default one =="
  COMBOS_DESC+=("--no-default-features");     COMBOS_FLAGS+=("--no-default-features")
else
  echo "== features found: ${FEATURES[*]} =="
  COMBOS_DESC+=("--no-default-features");     COMBOS_FLAGS+=("--no-default-features")
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS_DESC+=("--no-default-features --features $combo")
    COMBOS_FLAGS+=("--no-default-features --features $combo")
    COMBOS_DESC+=("default + --features $combo")
    COMBOS_FLAGS+=("--features $combo")
  done
fi

# ---------------------------------------------------------------------------
for profile_flag in "" "--release"; do
  prof_name="debug"; [ -n "$profile_flag" ] && prof_name="release"
  for idx in "${!COMBOS_FLAGS[@]}"; do
    desc="${COMBOS_DESC[$idx]}"
    flags="${COMBOS_FLAGS[$idx]}"
    echo
    echo "=== [$prof_name] $desc ==="

    # shellcheck disable=SC2086
    ( cd "$CRATE" && cargo build $CARGO_FLAGS $profile_flag $flags ) >/dev/null 2>&1 \
      || { echo "  BUILD FAILED"; fail=1; continue; }

    # Symbol parity for this exact build configuration.
    so="$CRATE/target/$prof_name/libdriver.so"
    c_syms=$(nm -D --defined-only --format=posix "$ROOT/c_src/build/libdriver.so" \
             | awk '{print $1}' | grep -v '^_' | sort -u)
    r_syms=$(nm -D --defined-only --format=posix "$so" \
             | awk '{print $1}' | grep -v '^_' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -n "$missing" ]; then
      echo "  SYMBOL DIFF NOT EMPTY -- missing from Rust .so:"; echo "$missing" | sed 's/^/    /'
      fail=1
    else
      echo "  symbol diff: empty ($(echo "$c_syms" | wc -l) symbols)"
    fi

    # shellcheck disable=SC2086
    if ( cd "$CRATE" && cargo test $CARGO_FLAGS $profile_flag $flags -- --test-threads=1 ) \
        > "${TMPDIR:-/tmp}/verify_${prof_name}_${idx}.log" 2>&1; then
      grep -h '^test result' "${TMPDIR:-/tmp}/verify_${prof_name}_${idx}.log" | sed 's/^/  /'
    else
      echo "  TESTS FAILED -- see ${TMPDIR:-/tmp}/verify_${prof_name}_${idx}.log"
      grep -E '^(test .*FAILED|failures:|DIVERGENCE)' \
        "${TMPDIR:-/tmp}/verify_${prof_name}_${idx}.log" | head -30 | sed 's/^/    /'
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
