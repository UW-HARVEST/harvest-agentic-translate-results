#!/usr/bin/env bash
# Phase D driver: run the full differential suite under EVERY cargo feature
# combination and under both build profiles.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded.
set -uo pipefail
cd "$(dirname "$0")"

fail=0

# ---- enumerate feature combinations --------------------------------------
mapfile -t FEATURES < <(
  python3 - <<'EOF'
import tomllib, pathlib
m = tomllib.loads(pathlib.Path('Cargo.toml').read_text())
feats = [f for f in m.get('features', {}) if f != 'default']
for f in feats:
    print(f)
EOF
)

echo "== declared features: ${#FEATURES[@]} (${FEATURES[*]-none})"

# Build the powerset of feature combinations (empty set == no-default-features).
COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

run() { # run <label> <cargo-args...>
  local label="$1"; shift
  local log="/tmp/harvest_cfg.log"
  echo
  echo "######## $label"
  if ! timeout 600 cargo build "$@" >"$log" 2>&1; then
    echo "!!!! BUILD FAILED: $label"; tail -20 "$log"; fail=1; return
  fi
  if ! timeout 600 cargo test "$@" >"$log" 2>&1; then
    echo "!!!! TESTS FAILED: $label"
    grep -E 'FAILED|panicked|^test .* \.\.\. FAILED|test result' "$log" | head -40
    fail=1
  fi
  # Report every test binary's result line, and the total count.
  grep -E '^(     Running|test result)' "$log" | sed 's/^/    /'
  local total
  total=$(grep -oE 'test result: ok\. [0-9]+' "$log" | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}')
  echo "    -> total passing tests: ${total}"
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    featargs=(--no-default-features)
    name="no-default-features"
  else
    featargs=(--no-default-features --features "$combo")
    name="features=$combo"
  fi

  # release profile is the shipping artifact (panic = "abort")
  HARVEST_RUST_SO="$PWD/target/release/libmerge_sort_lib.so" \
    run "release / $name" --release "${featargs[@]}"

  # debug profile: overflow checks on, panic = unwind — a genuinely different
  # code path for the wrapping arithmetic in the translation.
  HARVEST_RUST_SO="$PWD/target/debug/libmerge_sort_lib.so" \
    run "debug / $name" "${featargs[@]}"
done

# Also the plain default-features build.
HARVEST_RUST_SO="$PWD/target/release/libmerge_sort_lib.so" run "release / default" --release
HARVEST_RUST_SO="$PWD/target/debug/libmerge_sort_lib.so" run "debug / default"

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS AND PROFILES PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
