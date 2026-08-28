#!/usr/bin/env bash
# Enumerate every valid feature combination declared in Cargo.toml and run
# `cargo check` + `cargo test` for each, against the C shared library.
set -uo pipefail

cd "$(dirname "$0")"

LOGDIR=/tmp/xlate-verify
mkdir -p "$LOGDIR"

# ---- 1. build the C reference library -------------------------------------
(
  cd ../c_src
  mkdir -p build && cd build
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$LOGDIR/cmake.log" 2>&1 &&
    cmake --build . >>"$LOGDIR/cmake.log" 2>&1
) || {
  echo "C build FAILED - see $LOGDIR/cmake.log"
  tail -20 "$LOGDIR/cmake.log"
  exit 1
}
echo "C library built."

# ---- 2. enumerate feature combinations ------------------------------------
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re, sys, pathlib
txt = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^\[features\](.*?)(?=^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
for f in feats:
    print(f)
PY
)

COMBOS=()
if [ ${#FEATURES[@]} -eq 0 ]; then
  # No [features] table: the crate has exactly one build configuration.
  COMBOS+=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$b]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "Feature combinations (${#COMBOS[@]}): ${COMBOS[*]:-<default only>}"

# ---- 3. check + test each combination -------------------------------------
FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-default}"
  safe=$(echo "$label" | tr ',' '_')
  if [ -z "$combo" ]; then
    # No features declared, so the only distinct configuration is the empty
    # one; still pass --no-default-features to pin it explicitly.
    ARGS=(--no-default-features)
  else
    ARGS=(--no-default-features --features "$combo")
  fi

  for step in check test "test --release"; do
    # shellcheck disable=SC2086
    if ! timeout 600 cargo $step "${ARGS[@]}" >"$LOGDIR/${safe}-${step// /-}.log" 2>&1; then
      echo "FAIL  [$label] cargo $step  ($LOGDIR/${safe}-${step// /-}.log)"
      tail -30 "$LOGDIR/${safe}-${step// /-}.log"
      FAIL=1
    else
      echo "ok    [$label] cargo $step"
    fi
  done

  # ---- 4. symbol parity for this configuration ----------------------------
  # shellcheck disable=SC2086
  timeout 600 cargo build --release "${ARGS[@]}" >"$LOGDIR/${safe}-build.log" 2>&1
  C_SO=$(find ../c_src/build -maxdepth 1 -name '*.so' | head -1)
  R_SO=target/release/libarr_push_lib.so
  nm -D --defined-only "$C_SO" | awk '{print $3}' | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$' | sort -u >"$LOGDIR/c.syms"
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u >"$LOGDIR/r-${safe}.syms"
  if ! MISSING=$(comm -23 "$LOGDIR/c.syms" "$LOGDIR/r-${safe}.syms") || [ -n "$MISSING" ]; then
    echo "FAIL  [$label] Rust .so missing C symbols:"
    echo "$MISSING"
    FAIL=1
  else
    echo "ok    [$label] symbol parity"
  fi
done

if [ "$FAIL" -ne 0 ]; then
  echo "VERIFICATION FAILED"
  exit 1
fi
echo "All feature combinations verified."
