#!/usr/bin/env bash
# Run the whole differential suite against every build configuration of the
# Rust crate. Enumerates the feature combinations from Cargo.toml rather than
# assuming there are none.
set -uo pipefail
cd "$(dirname "$0")"

TIMEOUT=${TIMEOUT:-600}
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

step "Cargo features declared in Cargo.toml"
FEATURES=$(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            names.append(line.split('=')[0].strip())
print(' '.join(n for n in names if n != 'default'))
PY
)
if [ -z "$FEATURES" ]; then
  echo "none (crate has no [features] section) -> a single configuration"
  COMBOS=("default")
else
  echo "non-default features: $FEATURES"
  # Power set of the declared features, plus --no-default-features alone.
  COMBOS=("default" "none")
  set -- $FEATURES
  n=$#
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then
        eval "f=\${$((i+1))}"
        combo="${combo:+$combo,}$f"
      fi
    done
    COMBOS+=("$combo")
  done
fi
printf 'configurations to verify: %s\n' "${COMBOS[*]}"

step "Build the C shared library"
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls ../c_src/build/*.so | head -1)
echo "C .so: $C_SO"

run_suite() { # $1 = label, $2 = RUST_SO path, rest = cargo feature args
  local label="$1" so="$2"; shift 2
  step "cargo check ($label)"
  timeout "$TIMEOUT" cargo check --tests "$@" 2>&1 | tail -3 || { FAIL=1; return; }
  for t in phase_d_parity smoke phase_b_valid phase_c_errors fuzz_diff; do
    step "$label :: $t"
    RUST_SO="$so" timeout "$TIMEOUT" cargo test --test "$t" "$@" -- --test-threads=1 2>&1 \
      | grep -E 'test result|DIVERGENCE|panicked|FAILED' | head -20
    if [ "${PIPESTATUS[1]}" != "0" ]; then FAIL=1; echo ">>> $label :: $t FAILED"; fi
  done
}

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default) FARGS=() ;;
    none)    FARGS=(--no-default-features) ;;
    *)       FARGS=(--no-default-features --features "$combo") ;;
  esac

  step "Build the Rust .so (release, features=$combo)"
  timeout "$TIMEOUT" cargo build --release "${FARGS[@]}" 2>&1 | tail -3 \
    || { echo "release build FAILED"; FAIL=1; continue; }
  run_suite "release/$combo" "$PWD/target/release/libload_png_mem_lib.so" "${FARGS[@]}"

  step "Build the Rust .so (debug, features=$combo)"
  timeout "$TIMEOUT" cargo build "${FARGS[@]}" 2>&1 | tail -3 \
    || { echo "debug build FAILED"; FAIL=1; continue; }
  run_suite "debug/$combo" "$PWD/target/debug/libload_png_mem_lib.so" "${FARGS[@]}"
done

step "RESULT"
if [ "$FAIL" = "0" ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
