#!/usr/bin/env bash
# Build the C .so, then cargo check + cargo test the Rust crate for every valid
# feature combination declared in translation/Cargo.toml.
#
# Usage: ./verify_all.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGDIR=/tmp/xlate-verify
mkdir -p "$LOGDIR"
FAIL=0

echo "=== building C shared library ==="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
) >"$LOGDIR/cmake.log" 2>&1 || { echo "C build FAILED"; tail -20 "$LOGDIR/cmake.log"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C .so: $C_SO"

# ---- enumerate feature combinations from Cargo.toml [features] --------------
mapfile -t FEATURES < <(
  python3 - "$ROOT/translation/Cargo.toml" <<'PY'
import sys, re
text = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=', 1)[0].strip().strip('"')
        if name and name != 'default':
            names.append(name)
print('\n'.join(names), end='')
PY
)
# mapfile can yield a single empty element for empty input; drop blanks.
CLEANED=()
for f in ${FEATURES+"${FEATURES[@]}"}; do [ -n "$f" ] && CLEANED+=("$f"); done
FEATURES=(${CLEANED+"${CLEANED[@]}"})

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== no [features] declared: single configuration ==="
  COMBOS=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && combo+=("${FEATURES[i]}")
    done
    COMBOS+=("$(
      IFS=,
      echo "${combo[*]}"
    )")
  done
fi

run() { # run <label> <logfile> <cmd...>
  local label=$1 log=$2
  shift 2
  if timeout 600 "$@" >"$log" 2>&1; then
    echo "  PASS  $label"
  else
    echo "  FAIL  $label  (log: $log)"
    tail -30 "$log"
    FAIL=1
  fi
}

cd "$ROOT/translation"
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    label="<default>"
    slug="default"
    args=()
  else
    label="--no-default-features --features $combo"
    slug="${combo//,/_}"
    args=(--no-default-features --features "$combo")
  fi
  echo "=== configuration: $label ==="
  run "check  $label" "$LOGDIR/check-$slug.log" cargo check "${args[@]}"
  run "build  $label" "$LOGDIR/build-$slug.log" cargo build "${args[@]}"
  run "test   $label" "$LOGDIR/test-$slug.log" cargo test "${args[@]}"
done

# Also verify the default feature set (features on) still checks out.
if [ "${#FEATURES[@]}" -gt 0 ]; then
  echo "=== configuration: default features enabled ==="
  run "check  default-on" "$LOGDIR/check-defaulton.log" cargo check
  run "test   default-on" "$LOGDIR/test-defaulton.log" cargo test
fi

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAIL"
