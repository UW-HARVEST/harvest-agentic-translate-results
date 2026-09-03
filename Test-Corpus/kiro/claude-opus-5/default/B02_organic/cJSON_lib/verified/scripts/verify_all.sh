#!/usr/bin/env bash
# Phase D driver: build the C reference library, then run the whole
# differential suite for EVERY cargo feature combination and for both cargo
# profiles (the tests load the `.so` matching the profile they were built in,
# so dev and release are genuinely different configurations -- release also
# enables `panic = "abort"`).
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE="$ROOT/translation"
cd "$CRATE" || exit 1

fail=0

echo "=== building the C reference library ==="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /tmp/cjson_cmake.log 2>&1 \
    && cmake --build . > /tmp/cjson_cbuild.log 2>&1
) || { echo "C build FAILED (see /tmp/cjson_cbuild.log)"; exit 1; }
ls -1 "$ROOT"/c_src/build/*.so* | sed 's/^/  /'

# --- enumerate feature combinations --------------------------------------
# Read the [features] table from Cargo.toml (excluding "default").
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1]!="default" && a[1]!="") print a[1]}' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== no [features] table in Cargo.toml: 1 feature configuration ==="
  COMBOS+=("")                        # default (== only) configuration
  COMBOS+=("--no-default-features")   # must be identical, but verify it
else
  echo "=== features: ${FEATURES[*]} ==="
  COMBOS+=("")
  COMBOS+=("--no-default-features")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    set=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then set="$set,${FEATURES[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${set#,}")
  done
fi

for profile in dev release; do
  pflag=""
  [ "$profile" = release ] && pflag="--release"
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-default}]"
    echo
    echo "=== cargo check   $label ==="
    # shellcheck disable=SC2086
    if ! timeout 600 cargo check $pflag $combo > /tmp/cjson_check.log 2>&1; then
      echo "  CHECK FAILED"; tail -30 /tmp/cjson_check.log; fail=1; continue
    fi
    echo "  ok"

    echo "=== cargo build   $label ==="
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $pflag $combo > /tmp/cjson_build.log 2>&1; then
      echo "  BUILD FAILED"; tail -30 /tmp/cjson_build.log; fail=1; continue
    fi
    echo "  ok  ($(ls -l target/${profile/dev/debug}/libcJSON_test.so | awk '{print $5}') bytes)"

    echo "=== nm -D parity  $label ==="
    nm -D --defined-only "$ROOT/c_src/build/libcjson.so.1.7.19" | awk '{print $3}' | sort  > /tmp/cjson_c.txt
    nm -D --defined-only "$ROOT/c_src/build/libcJSON_test.so"   | awk '{print $3}' | sort >> /tmp/cjson_c.txt
    sort -u -o /tmp/cjson_c.txt /tmp/cjson_c.txt
    nm -D --defined-only "target/${profile/dev/debug}/libcJSON_test.so" | awk '{print $3}' | sort > /tmp/cjson_r.txt
    missing=$(comm -23 /tmp/cjson_c.txt /tmp/cjson_r.txt)
    if [ -n "$missing" ]; then
      echo "  MISSING SYMBOLS:"; echo "$missing" | sed 's/^/    /'; fail=1
    else
      echo "  ok  ($(wc -l < /tmp/cjson_c.txt) C symbols, all exported by Rust; 0 missing)"
    fi

    echo "=== cargo test    $label ==="
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test $pflag $combo > /tmp/cjson_test.log 2>&1; then
      echo "  TESTS FAILED"; grep -E "^(test |test result|---- |thread )" /tmp/cjson_test.log | tail -40; fail=1
    else
      grep -h "^test result" /tmp/cjson_test.log | sed 's/^/  /'
      printf "  total: %s passed, %s failed\n" \
        "$(grep -ho '^test result: ok\. [0-9]*' /tmp/cjson_test.log | awk '{s+=$4} END{print s+0}')" \
        "$(grep -hoE '[0-9]+ failed' /tmp/cjson_test.log | awk '{s+=$1} END{print s+0}')"
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
