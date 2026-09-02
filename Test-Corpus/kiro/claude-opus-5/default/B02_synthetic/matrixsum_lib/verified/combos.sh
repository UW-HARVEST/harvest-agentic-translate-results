#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY cargo feature
# combination declared by Cargo.toml.
#
# Feature names are extracted mechanically from the [features] section rather
# than hard-coded, so this stays correct if features are ever added.
set -uo pipefail
cd "$(dirname "$0")"

TIMEOUT=${TIMEOUT:-600}

# --- 1. Rebuild the C shared library (ground truth) -------------------------
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
echo "C .so: $(ls ../c_src/build/*.so)"

# --- 2. Enumerate feature names from [features] -----------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {gsub(/[ \t]/,"");split($0,a,"=");if(a[1]!="")print a[1]}' Cargo.toml
)
echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- 3. Build the combination list ------------------------------------------
# Always include: default, --no-default-features, and every subset of the
# declared features (2^n, capped for sanity).
COMBOS=()
COMBOS+=("--release")
COMBOS+=("--release --no-default-features")
n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${FEATURES[i]}"; fi
    done
    COMBOS+=("--release --no-default-features --features $sel")
    COMBOS+=("--release --features $sel")
  done
elif (( n > 12 )); then
  echo "WARNING: $n features -> 2^$n subsets; testing all-on / all-off only"
  all=$(IFS=,; echo "${FEATURES[*]}")
  COMBOS+=("--release --no-default-features --features $all")
  COMBOS+=("--release --features $all")
fi

echo "combinations to verify: ${#COMBOS[@]}"

# --- 4. cargo check + build + full test run per combination -----------------
fail=0
for combo in "${COMBOS[@]}"; do
  echo "=============================================================="
  echo ">>> cargo $combo"
  if ! timeout "$TIMEOUT" cargo check $combo >/tmp/chk.log 2>&1; then
    echo "!!! cargo check FAILED for [$combo]"; tail -20 /tmp/chk.log; fail=1; continue
  fi
  # The cdylib must exist for the tests to dlopen it.
  if ! timeout "$TIMEOUT" cargo build $combo >/tmp/bld.log 2>&1; then
    echo "!!! cargo build FAILED for [$combo]"; tail -20 /tmp/bld.log; fail=1; continue
  fi
  if ! timeout "$TIMEOUT" cargo test $combo -- --test-threads=1 >/tmp/tst.log 2>&1; then
    echo "!!! cargo test FAILED for [$combo]"; grep -E "^test .* FAILED|panicked|signal" /tmp/tst.log | head -20; fail=1; continue
  fi
  grep -E "^test result" /tmp/tst.log | sed 's/^/    /'
  echo "    OK [$combo]"
done

# --- 5. Symbol diff must be empty -------------------------------------------
echo "=============================================================="
C_SO=$(ls ../c_src/build/*.so | head -1)
RS_SO=target/release/libmatrixsum_lib.so
diff <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort | grep -vE '^_(_|Z)') \
     > /tmp/symdiff.txt
missing=$(grep -c '^<' /tmp/symdiff.txt)
echo "symbols exported by C but missing from Rust: $missing"
if (( missing != 0 )); then cat /tmp/symdiff.txt; fail=1; fi

echo "=============================================================="
if (( fail == 0 )); then echo "ALL FEATURE COMBINATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit $fail
