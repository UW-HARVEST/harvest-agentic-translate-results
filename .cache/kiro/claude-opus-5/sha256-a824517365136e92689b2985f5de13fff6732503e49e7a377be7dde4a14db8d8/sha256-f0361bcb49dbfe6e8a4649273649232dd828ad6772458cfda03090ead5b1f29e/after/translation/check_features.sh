#!/usr/bin/env bash
# Phase D automation:
#   1. rebuild both shared objects,
#   2. diff `nm -D` exported symbols C -> Rust (must be empty),
#   3. enumerate every feature combination from Cargo.toml and run
#      `cargo check` + the full differential suite for each.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

echo "### 1. build"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
cargo build --release --lib >/dev/null 2>&1 || { echo "Rust build FAILED"; exit 1; }

C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
R_SO="$ROOT/translation/target/release/libmemchra2_lib.so"
echo "C  .so: $C_SO"
echo "Rust .so: $R_SO"

echo
echo "### 2. symbol parity (nm -D)"
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only "$R_SO" | awk '{print $3}' \
  | grep -v -E '^_ZN|^rust_|^__rust|^_R' | sort -u > /tmp/r_syms.txt
echo "C exports:    $(wc -l < /tmp/c_syms.txt)"
echo "Rust exports: $(wc -l < /tmp/r_syms.txt)"
MISSING=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
if [ -n "$MISSING" ]; then
  echo "MISSING from Rust .so:"; echo "$MISSING"; SYM_FAIL=1
else
  echo "symbol diff (C -> Rust): EMPTY  [OK]"; SYM_FAIL=0
fi
echo "undefined non-libc symbols in Rust .so:"
nm -D --undefined-only "$R_SO" | awk '{print $2}' \
  | grep -v -E '@GLIBC|@GCC_|^_ITM_|^__cxa_|^__gmon|^__tls_get_addr|^$' \
  | sed 's/^/  /' || true
echo "  (empty above means none)"

echo
echo "### 3. feature combinations"
# Mechanically extract feature names from Cargo.toml's [features] section.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
NFEAT=$(printf '%s' "$FEATURES" | grep -c . || true)
echo "declared features: ${NFEAT:-0} ${FEATURES:-(none)}"

# Combination list: default, no-default, then every subset of the feature set.
run_combo() {
  local label="$1"; shift
  printf '  %-42s ' "$label"
  if ! cargo check --tests "$@" >/dev/null 2>&1; then echo "check FAILED"; return 1; fi
  if ! timeout 300 cargo test --release "$@" >/dev/null 2>&1; then echo "tests FAILED"; return 1; fi
  echo "OK"
}

COMBO_FAIL=0
run_combo "default" || COMBO_FAIL=1
run_combo "--no-default-features" --no-default-features || COMBO_FAIL=1

if [ "${NFEAT:-0}" -gt 0 ]; then
  # Enumerate all 2^n subsets.
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    if [ -z "$combo" ]; then continue; fi
    run_combo "--no-default-features --features $combo" \
      --no-default-features --features "$combo" || COMBO_FAIL=1
  done
else
  echo "  (no [features] section: default == --no-default-features == the only build)"
fi

echo
if [ "$SYM_FAIL" -eq 0 ] && [ "$COMBO_FAIL" -eq 0 ]; then
  echo "PHASE D: PASS"
  exit 0
fi
echo "PHASE D: FAIL (symbols=$SYM_FAIL combos=$COMBO_FAIL)"
exit 1
