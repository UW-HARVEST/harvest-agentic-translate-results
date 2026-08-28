#!/usr/bin/env bash
# Verify every build-time configuration of the crate.
#
# Cargo.toml declares no [features] table and CMakeLists.txt declares no
# options, so the only valid configuration is the default (empty) feature set.
# The loop is kept generic so added features are picked up automatically.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# Enumerate feature combinations from Cargo.toml. With no [features] table this
# yields exactly one combination: the empty one.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/[[:space:]]*=.*/,""); gsub(/[[:space:]]/,""); if ($0 != "default" && $0 != "") print}' Cargo.toml
)

COMBOS=("")
n=${#FEATURES[@]}
if (( n > 0 )); then
  COMBOS=()
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( bit = 0; bit < n; bit++ )); do
      if (( mask & (1 << bit) )); then
        combo="${combo:+$combo,}${FEATURES[bit]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "Feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<none>}'"; done
echo

C_SO=$(find ../c_src/build -maxdepth 1 -name 'lib*.so' | head -1)
if [[ -z "$C_SO" ]]; then
  echo "FAIL: C shared library not built. Run:"
  echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi
echo "C library: $C_SO"

status=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/none>}"
  for profile in dev release; do
    args=(--no-default-features)
    [[ -n "$combo" ]] && args+=(--features "$combo")
    [[ "$profile" == release ]] && args+=(--release)

    echo "=== [$label / $profile] cargo check ==="
    timeout 600 cargo check --all-targets "${args[@]}" 2>&1 | tail -3 || { echo "FAIL check"; status=1; continue; }

    echo "=== [$label / $profile] cargo build (cdylib) ==="
    timeout 600 cargo build "${args[@]}" 2>&1 | tail -3 || { echo "FAIL build"; status=1; continue; }

    outdir="target/$([[ $profile == release ]] && echo release || echo debug)"
    RS_SO="$outdir/libflip_horizontal_lib.so"

    echo "=== [$label / $profile] symbol comparison (nm -D) ==="
    nm -D --defined-only "$C_SO"  | awk '{print $3}' | grep -v '^$' | sort -u > /tmp/verify_c_syms.txt
    nm -D --defined-only "$RS_SO" | awk '{print $3}' | grep -v '^$' | sort -u > /tmp/verify_rs_syms.txt
    missing=$(comm -23 /tmp/verify_c_syms.txt /tmp/verify_rs_syms.txt)
    if [[ -n "$missing" ]]; then
      echo "FAIL: symbols exported by C but missing from Rust:"
      echo "$missing"
      status=1
    else
      echo "OK: all $(wc -l < /tmp/verify_c_syms.txt) C symbol(s) present in Rust .so"
    fi

    echo "=== [$label / $profile] cargo test ==="
    timeout 600 cargo test "${args[@]}" 2>&1 | tail -18 || { echo "FAIL test"; status=1; }
    echo
  done
done

if (( status == 0 )); then
  echo "ALL CONFIGURATIONS VERIFIED"
else
  echo "FAILURES DETECTED"
fi
exit $status
