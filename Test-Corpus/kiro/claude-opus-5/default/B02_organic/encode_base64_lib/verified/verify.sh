#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination x both profiles.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=../c_src/build/libdriver.so
fail=0

echo "=== Building the C shared library ==="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml (powerset of [features]).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  COMBOS=("")   # no [features] section -> exactly one combination: the empty set
else
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "=== Feature combinations to verify: ${#COMBOS[@]} (features found: ${n}) ==="
printf '  [%s]\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# For each combination: build both profiles, diff symbols, run the suite
# against BOTH the release .so and the debug .so (overflow checks on).
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  featflags=(--no-default-features)
  [[ -n "$combo" ]] && featflags+=(--features "$combo")
  label="${combo:-<no features>}"
  echo
  echo "############ combo: $label ############"

  for profile in release debug; do
    pflags=()
    [[ "$profile" == release ]] && pflags+=(--release)

    echo "--- build cdylib ($profile) ---"
    if ! timeout 600 cargo build "${pflags[@]}" "${featflags[@]}" >/dev/null 2>&1; then
      echo "BUILD FAILED: combo=$label profile=$profile"; fail=1; continue
    fi

    RS_SO="target/$profile/libdriver.so"

    echo "--- nm -D symbol parity ($profile) ---"
    diff <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort) \
      && echo "symbol diff: EMPTY (OK)" \
      || { echo "SYMBOL DIFF NON-EMPTY: combo=$label profile=$profile"; fail=1; }

    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort) \
      <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort))
    if [[ -n "$missing" ]]; then
      echo "MISSING FROM RUST: $missing"; fail=1
    fi

    echo "--- cargo test ($profile cdylib via DRIVER_RUST_SO) ---"
    if ! DRIVER_RUST_SO="$(pwd)/$RS_SO" timeout 600 cargo test --release "${featflags[@]}" 2>&1 \
         | grep -E "test result|FAILED|panicked"; then
      echo "TEST RUN PRODUCED NO RESULT LINE: combo=$label profile=$profile"; fail=1
    fi
    if DRIVER_RUST_SO="$(pwd)/$RS_SO" timeout 600 cargo test --release "${featflags[@]}" >/dev/null 2>&1; then
      echo "tests: PASS (combo=$label, rust .so=$profile)"
    else
      echo "tests: FAIL (combo=$label, rust .so=$profile)"; fail=1
    fi
  done
done

echo
if (( fail == 0 )); then
  echo "########## PHASE D: ALL COMBINATIONS PASSED ##########"
else
  echo "########## PHASE D: FAILURES PRESENT ##########"
fi
exit $fail
