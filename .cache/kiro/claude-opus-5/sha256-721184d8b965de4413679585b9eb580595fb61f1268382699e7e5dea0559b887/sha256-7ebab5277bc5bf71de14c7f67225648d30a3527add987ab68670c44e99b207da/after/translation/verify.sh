#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every feature
# combination declared in Cargo.toml.
#
# Usage: ./verify.sh
set -uo pipefail

cd "$(dirname "$0")"
here=$(pwd)
workdir=$(cd .. && pwd)

# ---------------------------------------------------------------------------
# 1. Build the C reference shared library (default configuration).
# ---------------------------------------------------------------------------
if ! ls "$workdir"/c_src/build/lib*.so >/dev/null 2>&1; then
  echo "== building C reference =="
  ( cd "$workdir/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/xlat_cmake.log 2>&1 \
    && cmake --build . >>/tmp/xlat_cmake.log 2>&1 ) \
    || { echo "C build failed; see /tmp/xlat_cmake.log"; exit 1; }
fi
ls -1 "$workdir"/c_src/build/lib*.so

# ---------------------------------------------------------------------------
# 2. Enumerate every feature combination from [features] in Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=("")   # the empty combination == --no-default-features
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== ${#FEATURES[@]} feature(s) declared: ${FEATURES[*]:-<none>} =="
echo "== ${#COMBOS[@]} combination(s) to verify =="

# ---------------------------------------------------------------------------
# 3. cargo check, then cargo test, for each combination and each profile.
# ---------------------------------------------------------------------------
status=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  echo
  echo "########## check: $label ##########"
  if ! timeout 600 cargo check "${args[@]}" --all-targets 2>&1 | tail -3; then
    echo "CHECK FAILED for $label"; status=1; continue
  fi

  for profile in "" "--release"; do
    echo "########## test: $label ${profile:-(debug)} ##########"
    if ! timeout 600 cargo test "${args[@]}" $profile 2>&1 \
         | grep -E '^(test [a-z_]+ \.\.\.|result:|---- |error)' ; then
      echo "TEST FAILED for $label ${profile:-(debug)}"; status=1
    fi
  done
done

# Also cover the crate's default feature set explicitly.
echo
echo "########## test: default features ##########"
timeout 600 cargo test --release 2>&1 | grep -E '^(result:|---- |error)' || status=1

# ---------------------------------------------------------------------------
# 4. Symbol export parity (also asserted from tests/exports.rs).
# ---------------------------------------------------------------------------
echo
echo "########## nm -D symbol comparison ##########"
c_so=$(ls -1t "$workdir"/c_src/build/lib*.so | head -1)
rs_so=$(ls -1t "$here"/target/release/libcheckshift_lib.so \
                "$here"/target/cdylib-under-test/release/libcheckshift_lib.so \
                2>/dev/null | head -1)
if [[ -z "${rs_so:-}" ]]; then
  cargo build --release >/dev/null 2>&1
  rs_so="$here/target/release/libcheckshift_lib.so"
fi
nm -D --defined-only "$c_so"  | awk '{print $3}' | sort -u > /tmp/xlat_c_syms.txt
nm -D --defined-only "$rs_so" | awk '{print $3}' | sort -u > /tmp/xlat_rs_syms.txt
missing=$(comm -23 /tmp/xlat_c_syms.txt /tmp/xlat_rs_syms.txt)
if [[ -n "$missing" ]]; then
  echo "MISSING FROM RUST .so:"; echo "$missing"; status=1
else
  echo "all $(wc -l < /tmp/xlat_c_syms.txt) C symbols are exported by the Rust .so"
fi

echo
if (( status == 0 )); then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit $status
