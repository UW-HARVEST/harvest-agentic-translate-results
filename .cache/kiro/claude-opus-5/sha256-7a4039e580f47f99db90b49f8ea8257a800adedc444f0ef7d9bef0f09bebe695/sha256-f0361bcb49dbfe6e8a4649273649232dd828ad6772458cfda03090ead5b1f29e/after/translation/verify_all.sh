#!/usr/bin/env bash
# Phase D automation: build both libraries, then run the differential suite
# across every feature combination x profile. Feature combos are extracted
# mechanically from Cargo.toml via `cargo metadata`.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=== Building C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
echo "C  .so: $C_SO"

# Enumerate feature combinations (powerset of declared features).
mapfile -t FEATURES < <(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys
d=json.load(sys.stdin)
fs=sorted(f for p in d["packages"] for f in p["features"] if f!="default")
[print(f) for f in fs]')
# Drop any empty lines so a feature-less crate yields a genuinely empty array.
TMP=(); for f in "${FEATURES[@]}"; do [ -n "$f" ] && TMP+=("$f"); done
FEATURES=("${TMP[@]+"${TMP[@]}"}")

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "No optional features declared -> only the default configuration exists."
  COMBOS+=("__default__" "__nodefault__")
else
  n=${#FEATURES[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("${combo:-__nodefault__}")
  done
  COMBOS+=("__default__")
fi

for profile in debug release; do
  for combo in "${COMBOS[@]}"; do
    args=()
    label="$combo"
    case "$combo" in
      __default__)   label="(default features)";;
      __nodefault__) label="(--no-default-features)"; args+=(--no-default-features);;
      *)             args+=(--no-default-features --features "$combo");;
    esac
    [ "$profile" = release ] && args+=(--release)

    echo
    echo "=== profile=$profile features=$label ==="

    # The cdylib the tests dlopen must match this configuration.
    buildargs=("${args[@]}")
    timeout 600 cargo build "${buildargs[@]}" >/dev/null 2>&1 \
      || { echo "  cdylib build FAILED"; FAIL=1; continue; }

    # Ensure the tests pick up THIS profile's cdylib: the test helper prefers
    # release/, so remove the other profile's artifact while testing debug.
    if [ "$profile" = debug ]; then
      mv -f target/release/libmax_size_frame_lib.so target/release/.hidden.so 2>/dev/null || true
    else
      mv -f target/release/.hidden.so target/release/libmax_size_frame_lib.so 2>/dev/null || true
    fi

    if timeout 600 cargo test "${args[@]}" 2>&1 | tail -3 | grep -q "test result: ok"; then
      echo "  PASS"
    else
      echo "  FAIL"
      FAIL=1
    fi

    mv -f target/release/.hidden.so target/release/libmax_size_frame_lib.so 2>/dev/null || true
  done
done

echo
echo "=== Symbol parity (nm -D) ==="
timeout 600 cargo build --release >/dev/null 2>&1
R_SO="target/release/libmax_size_frame_lib.so"
diff <(nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | sort) \
     <(nm -D --defined-only --format=posix "$R_SO" | awk '{print $1}' | sort) \
  && echo "  symbol sets IDENTICAL" || { echo "  symbol DIFF (above)"; FAIL=1; }

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$FAIL"
