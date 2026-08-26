#!/usr/bin/env bash
# Differential verification driver: builds the C reference .so, enumerates every
# Cargo feature combination, then runs cargo check + build + the full
# differential test suite for each one.
set -uo pipefail
cd "$(dirname "$0")"
LOG_DIR="${TMPDIR:-/tmp}"
fail=0

echo "== building C reference shared library =="
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . ) \
  > "$LOG_DIR/c-build.log" 2>&1 \
  || { echo "C BUILD FAILED"; tail -20 "$LOG_DIR/c-build.log"; exit 1; }
echo "   -> c_src/build/libdriver.so"

echo "== enumerating feature combinations =="
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, tomllib
feats = tomllib.load(open('Cargo.toml','rb')).get('features', {})
names = [f for f in feats if f != 'default']
for r in range(len(names) + 1):
    for c in itertools.combinations(names, r):
        print(",".join(c))
PY
)
echo "   ${#COMBOS[@]} combination(s): $(for c in "${COMBOS[@]}"; do printf '[%s] ' "$c"; done)"

for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  slug="$(echo "${combo:-none}" | tr ',' '_')"
  echo
  echo "=============== features: $label ==============="

  for step in "check --all-targets" "build"; do
    # shellcheck disable=SC2086
    if timeout 600 cargo $step --offline --no-default-features --features "$combo" \
         > "$LOG_DIR/cargo-$slug-${step%% *}.log" 2>&1; then
      echo "  cargo ${step%% *}: OK"
    else
      echo "  cargo ${step%% *}: FAILED"; tail -30 "$LOG_DIR/cargo-$slug-${step%% *}.log"; fail=1; continue 2
    fi
  done

  if timeout 600 cargo test --offline --no-default-features --features "$combo" \
       -- --test-threads=1 > "$LOG_DIR/test-$slug.log" 2>&1; then
    echo "  cargo test:  OK"
    grep -E "^test result:" "$LOG_DIR/test-$slug.log" | sed 's/^/    /'
  else
    echo "  cargo test:  FAILED"
    grep -E "^(test |failures:|---- )|panicked" "$LOG_DIR/test-$slug.log" | head -40 | sed 's/^/    /'
    fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$fail"
