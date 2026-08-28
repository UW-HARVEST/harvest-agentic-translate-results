#!/usr/bin/env bash
# Phase D: run the whole differential suite under EVERY feature combination.
#
# Features are enumerated from Cargo.toml rather than hard-coded, so this keeps
# working if a [features] table is ever added. With no [features] table the
# cross-product degenerates to the single default build.
set -uo pipefail
cd "$(dirname "$0")/.."

CARGO_FLAGS=(--offline)

# --- enumerate declared features -------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
if not m:
    sys.exit(0)
for line in m.group(1).splitlines():
    line = line.split('#', 1)[0].strip()
    if not line or '=' not in line:
        continue
    name = line.split('=', 1)[0].strip().strip('"')
    if name and name != "default":
        print(name)
PY
)

echo "declared features: ${FEATURES[*]:-<none>}"

# --- build the list of combinations ----------------------------------------
COMBOS=()
COMBOS+=("__default__")                       # default features
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("__none__")                        # --no-default-features
  COMBOS+=("__all__")                         # --all-features
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    set=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then set="$set,${FEATURES[i]}"; fi
    done
    COMBOS+=("${set#,}")
  done
fi

# --- ensure the C reference library exists ---------------------------------
C_SO=../c_src/build/libdriver.so
if [ ! -f "$C_SO" ]; then
  echo "building the C reference library"
  (cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null)
fi

fail=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) args=(); label="default features" ;;
    __none__)    args=(--no-default-features); label="--no-default-features" ;;
    __all__)     args=(--all-features); label="--all-features" ;;
    *)           args=(--no-default-features --features "$combo"); label="--features $combo" ;;
  esac

  printf '\n=== %s ===\n' "$label"
  if ! cargo build "${CARGO_FLAGS[@]}" "${args[@]}" >/dev/null 2>&1; then
    echo "  BUILD FAILED"
    cargo build "${CARGO_FLAGS[@]}" "${args[@]}" 2>&1 | tail -20
    fail=1
    continue
  fi
  # cargo check for good measure (catches warnings-as-errors style problems)
  if ! cargo check "${CARGO_FLAGS[@]}" "${args[@]}" --tests >/dev/null 2>&1; then
    echo "  CHECK FAILED"
    cargo check "${CARGO_FLAGS[@]}" "${args[@]}" --tests 2>&1 | tail -20
    fail=1
    continue
  fi
  if timeout 600 cargo test "${CARGO_FLAGS[@]}" "${args[@]}" -q 2>&1 | tail -25; then
    echo "  PASS"
  else
    echo "  TEST FAILED"
    fail=1
  fi

  # release profile too: it has panic=abort + overflow-checks=false, which is a
  # genuinely different code path for the wrapping pointer arithmetic.
  if ! cargo build --release "${CARGO_FLAGS[@]}" "${args[@]}" >/dev/null 2>&1; then
    echo "  RELEASE BUILD FAILED"
    fail=1
    continue
  fi
  if timeout 600 cargo test --release "${CARGO_FLAGS[@]}" "${args[@]}" -q 2>&1 | tail -25; then
    echo "  PASS (release)"
  else
    echo "  TEST FAILED (release)"
    fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASS (${#COMBOS[@]} combination(s), debug + release)"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
