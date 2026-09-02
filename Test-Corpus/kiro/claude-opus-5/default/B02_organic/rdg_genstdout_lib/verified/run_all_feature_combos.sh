#!/usr/bin/env bash
# Phase D driver: enumerate every cargo feature combination declared in
# Cargo.toml and run the whole differential suite for each, against both the
# debug and the release Rust cdylib (the release profile adds `panic = "abort"`
# and disables overflow checks, so it is a genuinely different build).
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
C_SO="$ROOT/../c_src/build/libdriver.so"

if [[ ! -f "$C_SO" ]]; then
  echo "building C shared object..."
  ( cd "$ROOT/../c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

# Enumerate features from the [features] table (excluding "default").
FEATURES=$(python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
print(' '.join(names))
PY
)

# Build the combination list: every subset of the optional features, plus the
# default build. With no [features] table this collapses to the two rows below.
COMBOS=()
COMBOS+=("DEFAULT")
COMBOS+=("NODEFAULT")
if [[ -n "${FEATURES// /}" ]]; then
  read -r -a FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if ((mask & (1 << b))); then combo+="${FARR[b]},"; fi
    done
    COMBOS+=("FEAT:${combo%,}")
  done
fi

echo "feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - $c"; done
echo

FAIL=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT) FLAGS=() ;;
    NODEFAULT) FLAGS=(--no-default-features) ;;
    FEAT:*) FLAGS=(--no-default-features --features "${combo#FEAT:}") ;;
  esac

  for profile in debug release; do
    PFLAGS=()
    [[ $profile == release ]] && PFLAGS=(--release)

    echo "=== cargo check   [$combo/$profile] ==="
    timeout 600 cargo check "${FLAGS[@]}" "${PFLAGS[@]}" >/tmp/fc_check.log 2>&1 \
      || { echo "CHECK FAILED"; tail -30 /tmp/fc_check.log; FAIL=1; continue; }

    echo "=== cargo build   [$combo/$profile] ==="
    timeout 600 cargo build "${FLAGS[@]}" "${PFLAGS[@]}" >/tmp/fc_build.log 2>&1 \
      || { echo "BUILD FAILED"; tail -30 /tmp/fc_build.log; FAIL=1; continue; }

    SO="$ROOT/target/$profile/libdriver.so"
    [[ -f "$SO" ]] || { echo "MISSING cdylib $SO"; FAIL=1; continue; }

    echo "--- nm -D symbol diff [$combo/$profile] ---"
    diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only "$SO"   | awk '$2=="T"||$2=="W"||$2=="D"{print $NF}' | sort -u) \
         > /tmp/fc_symdiff.txt
    if grep -q '^<' /tmp/fc_symdiff.txt; then
      echo "SYMBOL PARITY FAILED — missing from Rust .so:"; grep '^<' /tmp/fc_symdiff.txt; FAIL=1
    else
      echo "symbol parity OK (0 missing)"
    fi

    echo "=== cargo test    [$combo/$profile] (Rust .so = $profile) ==="
    DRIVER_RUST_SO="$SO" timeout 600 cargo test "${FLAGS[@]}" "${PFLAGS[@]}" \
      >/tmp/fc_test.log 2>&1
    rc=$?
    grep -E 'test result' /tmp/fc_test.log
    if [[ $rc -ne 0 ]]; then
      echo "TESTS FAILED [$combo/$profile]"; grep -E 'FAILED|panicked|assertion' /tmp/fc_test.log | head -20; FAIL=1
    fi
    echo
  done
done

if [[ $FAIL -eq 0 ]]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $FAIL
