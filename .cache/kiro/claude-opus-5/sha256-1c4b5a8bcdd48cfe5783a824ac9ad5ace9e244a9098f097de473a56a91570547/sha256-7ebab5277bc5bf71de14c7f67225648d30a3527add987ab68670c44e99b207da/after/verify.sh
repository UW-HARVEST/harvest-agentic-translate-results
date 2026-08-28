#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every valid
# build-time feature combination.
#
# `translation/Cargo.toml` declares no `[features]` table, so the only valid
# configuration is the empty feature set. This script derives that fact from
# Cargo.toml rather than hard-coding it, so it keeps working if features appear.
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$PWD
fail=0

# --- 1. Build the C shared library -------------------------------------------
mkdir -p c_src/build
(cd c_src/build \
  && timeout 300 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && timeout 300 cmake --build .) >/tmp/c-build.log 2>&1 \
  || { echo "C build FAILED (see /tmp/c-build.log)"; tail -20 /tmp/c-build.log; exit 1; }
C_SO=$(find c_src/build -maxdepth 2 -name '*.so' | sort | head -1)
echo "C shared library: $C_SO"

# --- 2. Enumerate feature combinations ---------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[ ]*=/ {
      split($0, kv, "=");
      gsub(/[ \t]/, "", kv[1]);
      if (kv[1] != "default") print kv[1];
  }
' translation/Cargo.toml | sort -u)

COMBOS=("")   # always test the empty (no-default-features) configuration
if [[ -n "$FEATURES" ]]; then
  mapfile -t FEATURE_LIST <<<"$FEATURES"
  n=${#FEATURE_LIST[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo+="${combo:+,}${FEATURE_LIST[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "Feature combinations to verify: ${#COMBOS[@]}"

# --- 3. check / test / symbol-compare each combination -----------------------
for combo in "${COMBOS[@]}"; do
  label=${combo:-<none>}
  echo
  echo "############ features: $label ############"
  featargs=(--no-default-features)
  [[ -n "$combo" ]] && featargs+=(--features "$combo")

  for profile in debug release; do
    relargs=()
    [[ $profile == release ]] && relargs+=(--release)

    if ! (cd translation && timeout 600 cargo check "${featargs[@]}" "${relargs[@]}") \
        >/tmp/check.log 2>&1; then
      echo "  [$profile] cargo check FAILED"; tail -20 /tmp/check.log; fail=1; continue
    fi
    echo "  [$profile] cargo check ok"

    if ! (cd translation && timeout 600 cargo test "${featargs[@]}" "${relargs[@]}") \
        >/tmp/test.log 2>&1; then
      echo "  [$profile] cargo test FAILED"; tail -30 /tmp/test.log; fail=1; continue
    fi
    echo "  [$profile] cargo test ok ($(grep -c '^test .* ok$' /tmp/test.log) tests passed)"

    # Symbol parity: every dynamic symbol the C .so defines must also be
    # defined by the Rust .so under the exact same name.
    (cd translation && timeout 600 cargo build --lib "${featargs[@]}" "${relargs[@]}") \
      >/tmp/build.log 2>&1
    R_SO=$(ls "translation/target/$profile/librev16_lib.so" \
              "translation/target/ffi-cdylib/$profile/librev16_lib.so" 2>/dev/null | head -1)
    if [[ -z ${R_SO:-} ]]; then
      echo "  [$profile] Rust cdylib not found"; fail=1; continue
    fi
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u))
    if [[ -n "$missing" ]]; then
      echo "  [$profile] MISSING EXPORTS from $R_SO:"; echo "$missing" | sed 's/^/    /'; fail=1
    else
      echo "  [$profile] symbol parity ok ($(nm -D --defined-only "$C_SO" | wc -l) C exports all present)"
    fi
  done
done

echo
if ((fail)); then echo "RESULT: FAILURES"; else echo "RESULT: all combinations verified"; fi
cd "$ROOT"
exit $fail
