#!/usr/bin/env bash
# Phase D driver: symbol parity + Phases B/C under every feature combination
# and every profile. Run from translation/.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
RUST_DEBUG="$CRATE/target/debug/librev16_lib.so"
RUST_RELEASE="$CRATE/target/release/librev16_lib.so"
fail=0

echo "=== Feature enumeration from Cargo.toml ==="
# Every declared feature name (excludes dependency tables).
FEATURES=$(awk '
  /^\[features\]/      {inf=1; next}
  /^\[/                {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' "$CRATE/Cargo.toml" | sort -u)
if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> combinations are: {default} and {no-default-features}"
else
  echo "$FEATURES"
fi

# Build the combination list: always the default build and the empty build;
# then the powerset of declared features (there are none here, but the loop
# generalises if features are added later).
COMBOS=("__default__" "__none__")
if [ -n "$FEATURES" ]; then
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    case "$combo" in
      __default__) featflags=() ; label="default" ;;
      __none__)    featflags=(--no-default-features) ; label="no-default-features" ;;
      *)           featflags=(--no-default-features --features "$combo") ; label="features=$combo" ;;
    esac
    prof_label="${profile_flag:-debug}"
    echo
    echo "=== [$prof_label / $label] cargo check ==="
    ( cd "$CRATE" && timeout 600 cargo check ${profile_flag:+$profile_flag} "${featflags[@]}" 2>&1 | tail -3 ) || fail=1

    echo "=== [$prof_label / $label] cargo build (cdylib) ==="
    ( cd "$CRATE" && timeout 600 cargo build ${profile_flag:+$profile_flag} "${featflags[@]}" 2>&1 | tail -2 ) || fail=1

    echo "=== [$prof_label / $label] symbol parity ==="
    RUST_SO="$RUST_DEBUG"; [ -n "$profile_flag" ] && RUST_SO="$RUST_RELEASE"
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u))
    if [ -n "$missing" ]; then
      echo "MISSING FROM RUST .so:"; echo "$missing"; fail=1
    else
      echo "symbol diff empty (C exports: $(nm -D --defined-only "$C_SO" | wc -l))"
    fi
    undef_non_libc=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
      | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^_Unwind_|^statx$|^gettid$' || true)
    if [ -n "$undef_non_libc" ]; then
      echo "UNDEFINED NON-LIBC SYMBOLS:"; echo "$undef_non_libc"; fail=1
    else
      echo "0 undefined non-libc symbols"
    fi

    echo "=== [$prof_label / $label] Phase B + C differential tests ==="
    ( cd "$CRATE" && timeout 600 cargo test ${profile_flag:+$profile_flag} "${featflags[@]}" 2>&1 \
        | grep -E '^test result|FAILED|panicked' ) || fail=1
  done
done

echo
if [ "$fail" -eq 0 ]; then echo "PHASE D: ALL COMBINATIONS PASS"; else echo "PHASE D: FAILURES PRESENT"; fi
exit "$fail"
