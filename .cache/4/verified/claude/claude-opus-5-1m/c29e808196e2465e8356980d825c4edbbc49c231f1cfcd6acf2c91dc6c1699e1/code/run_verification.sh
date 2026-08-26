#!/usr/bin/env bash
# Full verification driver: builds the C .so, then for EVERY cargo feature
# combination rebuilds the Rust cdylib (cargo test does NOT rebuild a cdylib on
# its own) and runs the differential suite.
set -euo pipefail
cd "$(dirname "$0")"

LOG_DIR="${TMPDIR:-/tmp}"

echo "### 1. Build the C shared library"
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$LOG_DIR/cmake.log" 2>&1 \
  && cmake --build . >>"$LOG_DIR/cmake.log" 2>&1)
ls -1 c_src/build/lib*.so

echo
echo "### 2. Enumerate feature combinations from Cargo.toml"
# Cargo.toml has no [features] table -> the only combination is the default one.
FEATURE_NAMES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' Cargo.toml)
if [ -z "$FEATURE_NAMES" ]; then
  echo "no [features] in Cargo.toml -> 1 combination (default/empty)"
  COMBOS=("")
else
  # power set of the declared features
  mapfile -t NAMES <<<"$FEATURE_NAMES"
  n=${#NAMES[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo="${combo:+$combo,}${NAMES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
printf 'combination: "%s"\n' "${COMBOS[@]}"

echo
echo "### 3. cargo check + build + test for every combination, debug and release"
status=0
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    if [ -n "$combo" ]; then feat=(--features "$combo"); else feat=(); fi
    label="features='${combo:-<none>}' profile='${profile:-debug}'"
    echo "--- cargo check   $label"
    cargo check --offline --no-default-features "${feat[@]}" $profile --tests >/dev/null
    echo "--- cargo build   $label   (mandatory: refreshes the cdylib under test)"
    cargo build --offline --no-default-features "${feat[@]}" $profile >/dev/null
    echo "--- cargo test    $label"
    if ! cargo test --offline --no-default-features "${feat[@]}" $profile -- --test-threads=4; then
      status=1
      echo "!!! FAILED: $label"
    fi
  done
done

echo
echo "### 4. Symbol parity (nm -D)"
C_SO=$(ls -1 c_src/build/lib*.so | head -1)
for rust_so in target/debug/libgaussian_kernel_lib.so target/release/libgaussian_kernel_lib.so; do
  [ -f "$rust_so" ] || continue
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '$2!="w" && $2!="V" {print $3}' | sort -u) \
    <(nm -D --defined-only "$rust_so" | awk '$2!="w" && $2!="V" {print $3}' | sort -u))
  if [ -n "$missing" ]; then
    echo "MISSING from $rust_so:"; echo "$missing"; status=1
  else
    echo "OK: $rust_so exports every C symbol"
  fi
done

exit $status
