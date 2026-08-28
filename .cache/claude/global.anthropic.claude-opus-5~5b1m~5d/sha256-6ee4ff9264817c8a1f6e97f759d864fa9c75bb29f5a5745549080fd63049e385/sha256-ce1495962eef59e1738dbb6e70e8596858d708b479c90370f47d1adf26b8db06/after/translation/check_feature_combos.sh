#!/usr/bin/env bash
# Phase D: run the whole differential suite for EVERY feature combination and
# EVERY build profile, plus the nm -D symbol-parity check for each.
#
# Feature names are read from Cargo.toml via `cargo metadata` (never hardcoded),
# and the powerset is enumerated automatically.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
cd "$here"

# ---- C ground truth (profile/feature independent) ------------------------
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
c_so="$root/c_src/build/libdriver.so"

# ---- enumerate the feature powerset ------------------------------------
FEATURES=()
while IFS= read -r f; do
  [[ -n $f ]] && FEATURES+=("$f")   # skip blank lines (no features declared)
done < <(
  cargo metadata --offline --no-deps --format-version 1 \
    | python3 -c 'import json,sys
feats=[f for f in json.load(sys.stdin)["packages"][0]["features"] if f!="default"]
print("\n".join(feats))'
)

COMBOS=()
COMBOS+=("")                        # default features
COMBOS+=("--no-default-features")   # nothing enabled
n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for ((mask=1; mask < (1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    COMBOS+=("--features $(IFS=,; echo "${sel[*]}")")
  done
fi

echo "declared (non-default) features: ${n} -> ${FEATURES[*]:-<none>}"
echo "combinations to verify: ${#COMBOS[@]}"
echo

fail=0
for profile in debug release; do
  prof_flag=""
  [[ $profile == release ]] && prof_flag="--release"
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-<default>}]"
    echo "################################################################"
    echo "## $label"
    echo "################################################################"

    # shellcheck disable=SC2086
    if ! cargo build --offline $prof_flag $combo >/dev/null 2>&1; then
      echo "FAIL: cargo build ($label)"; fail=1; continue
    fi

    rust_so="$here/target/$profile/libdriver.so"
    if [[ ! -f $rust_so ]]; then
      echo "FAIL: $rust_so not produced ($label)"; fail=1; continue
    fi

    # --- symbol parity: every symbol the C exports must be exported by Rust
    if diff <(nm -D --defined-only "$c_so"   | awk '{print $3}' | sort) \
            <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort) >/dev/null; then
      echo "symbols: OK (diff empty)"
    else
      echo "FAIL: symbol diff non-empty ($label)"
      diff <(nm -D --defined-only "$c_so"   | awk '{print $3}' | sort) \
           <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort) || true
      fail=1
    fi

    # --- differential tests
    # shellcheck disable=SC2086
    if cargo test --offline $prof_flag $combo -- --test-threads=4 2>&1 | tail -n 25; then
      echo "tests: OK ($label)"
    else
      echo "FAIL: cargo test ($label)"; fail=1
    fi
    echo
  done
done

echo "================================================================"
if (( fail )); then
  echo "RESULT: FAILURES PRESENT"
  exit 1
fi
echo "RESULT: all feature combinations x profiles PASSED"
