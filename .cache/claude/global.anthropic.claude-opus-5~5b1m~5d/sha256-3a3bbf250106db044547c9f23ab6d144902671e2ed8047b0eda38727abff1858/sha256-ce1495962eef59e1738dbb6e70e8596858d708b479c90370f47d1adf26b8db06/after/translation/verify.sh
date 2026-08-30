#!/usr/bin/env bash
# Full verification run: builds the C reference library, then runs the
# differential test suite under every Cargo feature combination.
#
# Usage: cd translation && ./verify.sh
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
CARGO_FLAGS=(--offline)
rc=0

echo "== building the C reference shared library =="
(
  cd "$ROOT/../c_src" &&
  mkdir -p build && cd build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
ls -l "$ROOT/../c_src/build/libdriver.so"

echo
echo "== enumerating cargo feature combinations =="
# Every feature declared in [features] (excluding "default").
FEATURES=$(python3 - "$ROOT/Cargo.toml" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
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
  echo "no [features] declared -> the only combinations are default / --no-default-features"
  COMBOS=("" "--no-default-features")
else
  echo "features: $FEATURES"
  COMBOS=("" "--no-default-features")
  # Power set of the declared features.
  read -r -a FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FARR[$i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    COMBOS+=("--features $(IFS=,; echo "${sel[*]}")")
  done
fi

for combo in "${COMBOS[@]}"; do
  label=${combo:-"(default features)"}
  echo
  echo "== cargo test ${label} =="
  # shellcheck disable=SC2086
  if timeout 600 cargo test "${CARGO_FLAGS[@]}" $combo 2>&1 | tail -n 6; then
    echo "PASS: ${label}"
  else
    echo "FAIL: ${label}"
    rc=1
  fi
done

echo
echo "== symbol diff (must be empty) =="
diff <(nm -D --defined-only "$ROOT/../c_src/build/libdriver.so" | awk '{print $NF}' | sort -u) \
     <(nm -D --defined-only "$ROOT/target/so-under-test/release/libdriver.so" | awk '{print $NF}' | sort -u) \
  && echo "symbol diff empty: OK" || { echo "SYMBOL DIFF NOT EMPTY"; rc=1; }

echo
if [ "$rc" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$rc"
