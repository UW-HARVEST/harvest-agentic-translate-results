#!/usr/bin/env bash
# Full verification matrix: build the C .so and the Rust .so, then run the whole
# differential suite for every feature combination and every build profile.
set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

echo "=== building C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
C_SO=$(find "$ROOT/c_src/build" -name '*.so' | head -1)
echo "C  .so: $C_SO"

# --- enumerate feature combinations -----------------------------------------
# Read the [features] table out of Cargo.toml; the powerset of the declared
# features is the set of combinations to verify. With no [features] table the
# only combination is the empty one.
mapfile -t FEATURES < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name and name != 'default':
                print(name)
PY
)

COMBOS=("")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then combo="${combo:+$combo,}${FEATURES[$b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "=== feature combinations: ${#COMBOS[@]} (declared features: ${FEATURES[*]:-<none>}) ==="

fail=0
for profile in release dev; do
  flag=""; outdir="debug"
  if [ "$profile" = "release" ]; then flag="--release"; outdir="release"; fi
  for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then feat_args=(--no-default-features); label="<no features>";
    else feat_args=(--no-default-features --features "$combo"); label="$combo"; fi

    echo
    echo "=== profile=$profile features=$label ==="
    cargo build $flag "${feat_args[@]}" >/dev/null 2>&1
    R_SO="target/$outdir/libgotomach_lib.so"
    [ -f "$R_SO" ] || { echo "MISSING $R_SO"; fail=1; continue; }

    if GOTOMACH_C_SO="$C_SO" GOTOMACH_RUST_SO="$(pwd)/$R_SO" \
       timeout 600 cargo test $flag "${feat_args[@]}" --no-fail-fast -- --test-threads=1 2>&1 \
       | grep -E '^(test result|running|error)'; then
      :
    fi
    # shellcheck disable=SC2181
    if ! GOTOMACH_C_SO="$C_SO" GOTOMACH_RUST_SO="$(pwd)/$R_SO" \
         timeout 600 cargo test $flag "${feat_args[@]}" --no-fail-fast -- --test-threads=1 >/tmp/run_all.log 2>&1; then
      echo "FAILED: profile=$profile features=$label"
      grep -E '^test .* FAILED|panicked at' /tmp/run_all.log | head -20
      fail=1
    fi
  done
done

# Also run with the default feature set (identical to <no features> here, but
# exercised explicitly so a future [features] table cannot silently skip it).
for profile in release dev; do
  flag=""; outdir="debug"
  if [ "$profile" = "release" ]; then flag="--release"; outdir="release"; fi
  echo
  echo "=== profile=$profile features=<default> ==="
  cargo build $flag >/dev/null 2>&1
  if ! GOTOMACH_C_SO="$C_SO" GOTOMACH_RUST_SO="$(pwd)/target/$outdir/libgotomach_lib.so" \
       timeout 600 cargo test $flag --no-fail-fast -- --test-threads=1 >/tmp/run_all.log 2>&1; then
    echo "FAILED: profile=$profile features=<default>"
    grep -E '^test .* FAILED|panicked at' /tmp/run_all.log | head -20
    fail=1
  else
    grep -E '^test result' /tmp/run_all.log
  fi
done

echo
echo "=== symbol diff (C defined - Rust defined), must be empty ==="
cargo build --release >/dev/null 2>&1
diff <(nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | sort) \
     <(nm -D --defined-only --format=posix target/release/libgotomach_lib.so | awk '{print $1}' | sort) \
     | grep '^<' && { echo "SYMBOL PARITY FAILED"; fail=1; } || echo "no missing symbols"

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; exit 1; fi
