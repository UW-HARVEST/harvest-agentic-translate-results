#!/usr/bin/env bash
# Phase D — enumerate the Cargo feature powerset mechanically and run the full
# differential suite for every element, against BOTH the debug and the release
# cdylib.
#
# `cargo test` does not relink the cdylib, so every combination must be
# `cargo build`-ed first; the harness also asserts freshness and will fail loudly
# if that is ever skipped.
set -uo pipefail
cd "$(dirname "$0")"

TIMEOUT=${TIMEOUT:-600}
fail=0

# ---------------------------------------------------------------------------
# Enumerate features declared in Cargo.toml (the [features] table only).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "== features declared in Cargo.toml: ${#FEATURES[@]} =="
for f in "${FEATURES[@]:-}"; do [ -n "$f" ] && echo "   - $f"; done

# Build the powerset of feature combinations as --features strings.
COMBOS=("")   # the empty set == --no-default-features
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo+="${FEATURES[i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
fi

# ---------------------------------------------------------------------------
# run <label> <extra cargo args...>
# ---------------------------------------------------------------------------
run() {
  local label="$1"; shift
  echo
  echo "############################################################"
  echo "# $label"
  echo "############################################################"

  for profile_flag in "" "--release"; do
    local pdir="debug"; [ -n "$profile_flag" ] && pdir="release"
    echo "--- cargo build $profile_flag [$label] ---"
    if ! timeout "$TIMEOUT" cargo build $profile_flag "$@" >/tmp/sl_build.log 2>&1; then
      echo "BUILD FAILED ($label $pdir)"; tail -30 /tmp/sl_build.log; fail=1; continue
    fi
    echo "--- cargo test $profile_flag [$label] (against target/$pdir cdylib) ---"
    if STATICLOOP_RUST_SO="$PWD/target/$pdir/libStaticLoop.so" \
       timeout "$TIMEOUT" cargo test $profile_flag "$@" 2>&1 | tee /tmp/sl_test.log | \
       grep -E '^(test result|error|warning: unused)' ; then :; fi
    if grep -qE '^test result: FAILED|error\[|error:' /tmp/sl_test.log; then
      echo "TEST FAILED ($label $pdir)"; grep -E 'FAILED|diverged|panicked' /tmp/sl_test.log | head -20; fail=1
    fi
  done
}

# Default configuration (default features on).
run "default features"

# Every element of the feature powerset (with default features disabled).
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    run "--no-default-features" --no-default-features
  else
    run "--no-default-features --features $combo" --no-default-features --features "$combo"
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS x BOTH PROFILES PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$fail"
