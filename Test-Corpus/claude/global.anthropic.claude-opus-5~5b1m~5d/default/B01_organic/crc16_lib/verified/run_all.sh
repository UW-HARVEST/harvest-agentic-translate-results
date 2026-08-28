#!/usr/bin/env bash
# Full verification driver: builds the C .so, then runs the differential suite
# under EVERY cargo feature combination (enumerated from Cargo.toml, not
# hardcoded).
#
# Usage: ./run_all.sh [--with-mutations]
set -uo pipefail
cd "$(dirname "$0")" || exit 1

CARGO_OFFLINE=${CARGO_OFFLINE:---offline}
fail=0

echo "############ 1. Build the C shared library ############"
( cd ../c_src \
  && cmake -S . -B build -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build build >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find ../c_src/build -maxdepth 1 -name 'lib*.so' | head -1)
echo "C .so: $C_SO"

echo
echo "############ 2. Enumerate feature combinations ############"
# Every feature declared in [features], excluding the implicit "default".
mapfile -t RAW_FEATURES < <(python3 - <<'PY'
import re, sys
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip().strip('"')
        if name and name != "default":
            feats.append(name)
print("\n".join(feats))
PY
)
# `mapfile` yields a single empty element for empty input — drop blanks.
FEATURES=()
for f in ${RAW_FEATURES+"${RAW_FEATURES[@]}"}; do
  [ -n "$f" ] && FEATURES+=("$f")
done

# Build the list of (label, cargo-flags) combos: the full power set of features,
# plus the default build. With no [features] at all this correctly reduces to the
# two configurations that actually exist.
COMBOS=()
COMBOS+=("default|")
COMBOS+=("no-default-features|--no-default-features")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  echo "declared features (${n}): ${FEATURES[*]}"
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("no-default+$combo|--no-default-features --features $combo")
    COMBOS+=("default+$combo|--features $combo")
  done
else
  echo "no [features] section in Cargo.toml -> the crate has exactly one"
  echo "feature configuration; --no-default-features is verified as well."
fi
echo "combinations to verify: ${#COMBOS[@]}"

echo
echo "############ 3. Verify each combination ############"
for entry in "${COMBOS[@]}"; do
  label="${entry%%|*}"
  flags="${entry#*|}"
  echo "----------------------------------------------------------------"
  echo ">>> [$label]  cargo flags: '${flags:-<none>}'"

  # shellcheck disable=SC2086
  if ! cargo build --release $CARGO_OFFLINE $flags >/dev/null 2>&1; then
    echo "    BUILD FAILED"; fail=1; continue
  fi
  # shellcheck disable=SC2086
  if ! cargo check $CARGO_OFFLINE $flags --tests >/dev/null 2>&1; then
    echo "    CHECK FAILED"; fail=1; continue
  fi
  # shellcheck disable=SC2086
  out=$(timeout 600 cargo test $CARGO_OFFLINE $flags --tests 2>&1)
  if [ $? -ne 0 ]; then
    echo "    TESTS FAILED"; echo "$out" | grep -E "^(test |error|thread)" | head -20; fail=1
  else
    echo "$out" | grep -E "test result" | sed 's/^/    /'
    total=$(echo "$out" | grep -oE '[0-9]+ passed' | awk '{s+=$1} END {print s}')
    echo "    OK — $total tests passed under [$label]"
  fi
done

if [ "${1:-}" = "--with-mutations" ]; then
  echo
  echo "############ 4. Mutation testing (suite sensitivity) ############"
  ./mutation_check.sh || fail=1
fi

echo
echo "################################################################"
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
