#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and run `cargo check` and
# `cargo test` for each. Also re-runs the whole suite against the optimized
# release cdylib, and re-checks C/Rust export parity.
#
# Usage: ./check_all_features.sh [--tests]
set -uo pipefail
cd "$(dirname "$0")"

TIMEOUT=${TIMEOUT:-600}
RUN_TESTS=0
[[ "${1:-}" == "--tests" ]] && RUN_TESTS=1

# --- Enumerate features declared in Cargo.toml -----------------------------
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
if not m:
    sys.exit(0)
for line in m.group(1).splitlines():
    line = line.split('#')[0].strip()
    if not line or '=' not in line:
        continue
    name = line.split('=')[0].strip()
    if name and name != 'default':
        print(name)
PY
)

if [[ ${#FEATURES[@]} -eq 0 ]]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  COMBOS=("")
else
  echo "Declared features: ${FEATURES[*]}"
  COMBOS=()
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("$(IFS=,; echo "${sel[*]}")")
  done
fi

fail=0
run() { # run <label> <cmd...>
  local label="$1"; shift
  printf '%-58s' "$label"
  if timeout "$TIMEOUT" "$@" >/tmp/cf.log 2>&1; then
    echo "OK"
  else
    echo "FAIL"
    tail -n 25 /tmp/cf.log
    fail=1
  fi
}

for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    run "check [default]"               cargo check
    run "check [--no-default-features]" cargo check --no-default-features
    run "build --release [default]"     cargo build --release
    if [[ $RUN_TESTS -eq 1 ]]; then
      run "test  [default]"             cargo test
      CAPSULE_RUST_SO="$PWD/target/release/libcapsule_lib.so" \
        run "test  [default, release cdylib]" cargo test
    fi
  else
    run "check [$combo]" cargo check --no-default-features --features "$combo"
    run "build --release [$combo]" \
      cargo build --release --no-default-features --features "$combo"
    if [[ $RUN_TESTS -eq 1 ]]; then
      run "test  [$combo]" cargo test --no-default-features --features "$combo"
    fi
  fi
done

# --- Export parity ---------------------------------------------------------
C_SO=$(ls ../c_src/build/*.so 2>/dev/null | grep -v capsule_lib | head -1)
if [[ -n "$C_SO" ]]; then
  for RS_SO in target/release/libcapsule_lib.so target/ffi-so/debug/libcapsule_lib.so; do
    [[ -f "$RS_SO" ]] || continue
    nm -D --defined-only "$C_SO"  | awk '{print $3}' | grep -v '^_' | sort -u > /tmp/c_syms
    nm -D --defined-only "$RS_SO" | awk '{print $3}' | grep -v '^_' | sort -u > /tmp/r_syms
    missing=$(comm -23 /tmp/c_syms /tmp/r_syms)
    printf '%-58s' "exports: $(basename "$RS_SO") vs C ($(wc -l < /tmp/c_syms) syms)"
    if [[ -z "$missing" ]]; then
      echo "OK"
    else
      echo "FAIL"
      echo "missing from Rust .so:"; echo "$missing"
      fail=1
    fi
  done
else
  echo "WARNING: no C .so found under ../c_src/build (build it first)"
fi

echo
[[ $fail -eq 0 ]] && echo "ALL CONFIGURATIONS PASS" || echo "FAILURES PRESENT"
exit $fail
