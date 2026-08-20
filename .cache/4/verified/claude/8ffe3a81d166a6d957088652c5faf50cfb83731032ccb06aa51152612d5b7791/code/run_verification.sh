#!/usr/bin/env bash
# Full verification driver: every build configuration x Phase B + Phase C.
#
#   ./run_verification.sh
#
# Phase A artifacts: SYMBOLS.md, ERRORS.md, CONFIGS.md
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"
fail=0
note() { printf '\n\033[1m==== %s ====\033[0m\n' "$*"; }

# ---------------------------------------------------------------------------
# Enumerate every valid feature combination from Cargo.toml.
# ---------------------------------------------------------------------------
note "Phase A: enumerating feature combinations"
FEATURES=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
print(' '.join(names))
PY
)
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features] -> the only combination is the default (empty) one."
  COMBOS=("")
else
  echo "features: $FEATURES"
  COMBOS=()
  arr=($FEATURES); n=${#arr[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${arr[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "combination count: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# Build the C reference shared library.
# ---------------------------------------------------------------------------
note "Building the C reference .so"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls c_src/build/*.so | head -1)
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# cargo check + build + test for every combination, in both profiles.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then FEAT_ARGS=(--no-default-features); label="<no-default-features>";
  else FEAT_ARGS=(--no-default-features --features "$combo"); label="$combo"; fi

  for profile in dev release; do
    if [ "$profile" = release ]; then PROF_ARGS=(--release); else PROF_ARGS=(); fi
    note "combo=$label profile=$profile : cargo check"
    timeout 600 cargo check $CARGO_FLAGS "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" --all-targets \
      || { echo "CHECK FAILED ($label/$profile)"; fail=1; continue; }

    note "combo=$label profile=$profile : cargo build (cdylib)"
    # `cargo test` does not rebuild a cdylib-only lib target, so build it first.
    timeout 600 cargo build $CARGO_FLAGS "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" \
      || { echo "BUILD FAILED ($label/$profile)"; fail=1; continue; }

    note "combo=$label profile=$profile : Phase D symbol parity"
    if [ "$profile" = release ]; then RS_SO=target/release/libbuffapp_lib.so; else RS_SO=target/debug/libbuffapp_lib.so; fi
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u))
    if [ -n "$missing" ]; then
      echo "SYMBOL PARITY FAILED - missing from Rust .so:"; echo "$missing"; fail=1
    else
      echo "symbol diff empty: all $(nm -D --defined-only "$C_SO" | wc -l) C symbols exported by $RS_SO"
    fi

    note "combo=$label profile=$profile : Phase B + Phase C differential tests"
    timeout 600 cargo test $CARGO_FLAGS "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" \
      || { echo "TESTS FAILED ($label/$profile)"; fail=1; }
  done
done

note "RESULT"
if [ "$fail" = 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $fail
