#!/usr/bin/env bash
# Check and test the crate against the C library for EVERY valid Cargo feature
# combination.
#
# Feature names are read from `[features]` in Cargo.toml (excluding `default`),
# and every subset of them is exercised. If the crate declares no features the
# single "no features" configuration is used.
set -uo pipefail

cd "$(dirname "$0")"

# --- enumerate features -----------------------------------------------------
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re, sys
text = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', text, re.M | re.S)
if not m:
    sys.exit(0)
for line in m.group(1).splitlines():
    line = line.split('#', 1)[0].strip()
    if not line or '=' not in line:
        continue
    name = line.split('=', 1)[0].strip().strip('"')
    if name != 'default':
        print(name)
PY
)

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  COMBOS=("")
else
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo+="${FEATURES[i]},"
      fi
    done
    COMBOS+=("${combo%,}")
  done
fi

echo "features declared: ${n} (${FEATURES[*]-none})"
echo "combinations to verify: ${#COMBOS[@]}"

# --- build the C reference library ------------------------------------------
C_BUILD="../c_src/build"
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > /tmp/c-build.log 2>&1 \
  || { echo "C build FAILED, see /tmp/c-build.log"; tail -20 /tmp/c-build.log; exit 1; }
echo "C reference library: $C_BUILD/libdriver.so"

# --- verify every combination ----------------------------------------------
rc=0
for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    args=(--no-default-features)
    label="<no features>"
  else
    args=(--no-default-features --features "$combo")
    label="$combo"
  fi

  echo
  echo "=============================================================="
  echo "combination: $label"
  echo "=============================================================="

  log="/tmp/check-${combo//,/_}.log"
  if timeout 600 cargo check "${args[@]}" --all-targets > "$log" 2>&1; then
    echo "  cargo check : ok"
  else
    echo "  cargo check : FAILED (see $log)"
    tail -30 "$log"
    rc=1
    continue
  fi

  # The nested build inside the test harness must use the same features.
  export DRIVER_SO_FEATURE_ARGS="${args[*]}"
  log="/tmp/test-${combo//,/_}.log"
  if timeout 600 cargo test "${args[@]}" > "$log" 2>&1; then
    echo "  cargo test  : ok"
    grep -E "^test result:" "$log" | sed 's/^/    /'
  else
    echo "  cargo test  : FAILED (see $log)"
    grep -E "^test .*(FAILED|panicked)|^assertion|mismatch" "$log" | head -20 | sed 's/^/    /'
    rc=1
  fi

  # Symbol parity (also asserted by tests/symbols.rs; shown here for the log).
  rm -rf target/ffi-so
  timeout 600 cargo build --lib "${args[@]}" --target-dir target/ffi-so > /dev/null 2>&1
  if diff <(nm -D --defined-only "$C_BUILD/libdriver.so" | awk '{print $NF}' | sort) \
          <(nm -D --defined-only target/ffi-so/debug/libdriver.so | awk '{print $NF}' | sort) \
          > /tmp/symdiff.txt 2>&1; then
    echo "  symbols     : identical"
  else
    echo "  symbols     : differ (C-only lines starting with '<' are failures)"
    sed 's/^/    /' /tmp/symdiff.txt
  fi
  unset DRIVER_SO_FEATURE_ARGS
done

echo
if (( rc == 0 )); then
  echo "ALL ${#COMBOS[@]} COMBINATION(S) PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$rc"
