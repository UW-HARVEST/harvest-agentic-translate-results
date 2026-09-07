#!/usr/bin/env bash
# Completion gate for the C-to-Rust `pow` verification.
#
# Rebuilds both shared objects, checks symbol parity mechanically, then runs the
# whole differential suite under every Cargo feature combination and in both the
# debug and release profiles (the Rust `.so` is optimised in release, which is
# where an errno-ordering mistake would show up).
#
# Usage: ./verify.sh          (run from translation/)
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
C_SO="$ROOT/c_src/build/libpow.so"
TIMEOUT=600
fail=0

step() { printf '\n=== %s ===\n' "$1"; }

step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout $TIMEOUT cmake --build . >/dev/null )
test -f "$C_SO"
echo "built $C_SO"

step "Enumerating feature combinations"
# Every combination of the crate's optional features, plus the default build.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{print $1}' "$HERE/Cargo.toml"
)
COMBOS=("__default__")
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("$(IFS=,; echo "${sel[*]}")")
  done
fi
printf 'features declared: %s\n' "${FEATURES[*]:-<none>}"
printf 'combinations to test: %s\n' "${#COMBOS[@]}"

for profile in debug release; do
  relflag=(); [[ $profile == release ]] && relflag=(--release)

  for combo in "${COMBOS[@]}"; do
    if [[ $combo == "__default__" ]]; then
      featflag=(); label="default features"
    elif [[ -z $combo ]]; then
      featflag=(--no-default-features); label="no features"
    else
      featflag=(--no-default-features --features "$combo"); label="features: $combo"
    fi

    step "$profile / $label"

    if ! timeout $TIMEOUT cargo build "${relflag[@]}" "${featflag[@]}" >/dev/null 2>&1; then
      echo "FAIL: cargo build ($profile, $label)"; fail=1; continue
    fi
    RUST_SO="$HERE/target/$profile/libpow.so"
    test -f "$RUST_SO"

    # --- symbol parity: every symbol the C .so defines must be defined by Rust
    missing="$(comm -23 \
      <(nm -D "$C_SO"   | awk '$0 !~ / U /{print $NF}' | sort -u) \
      <(nm -D "$RUST_SO" | awk '$0 !~ / U /{print $NF}' | sort -u))"
    if [[ -n $missing ]]; then
      echo "FAIL: symbols defined by the C .so but missing from the Rust .so:"
      echo "$missing"; fail=1
    else
      echo "symbol parity: OK (0 missing)"
    fi

    # --- the C .so's non-libc imports must also be satisfiable
    if ! nm -D "$RUST_SO" | grep -q ' T my_pow$'; then
      echo "FAIL: my_pow is not an exported text symbol of the Rust .so"; fail=1
    fi

    # --- full differential suite, both .so files loaded via libloading
    if POW_C_SO="$C_SO" POW_RUST_SO="$RUST_SO" \
       timeout $TIMEOUT cargo test "${relflag[@]}" "${featflag[@]}" 2>&1 \
       | grep -vE '^(Domain|Range) error' | grep -E 'test result|FAILED|^error'; then
      :
    fi
    if ! POW_C_SO="$C_SO" POW_RUST_SO="$RUST_SO" \
         timeout $TIMEOUT cargo test "${relflag[@]}" "${featflag[@]}" >/tmp/pow_test_$$.log 2>&1; then
      echo "FAIL: differential suite ($profile, $label)"
      grep -vE '^(Domain|Range) error' /tmp/pow_test_$$.log | tail -40
      fail=1
    fi
    rm -f /tmp/pow_test_$$.log
  done
done

step "Result"
if ((fail)); then
  echo "VERIFICATION FAILED"
  exit 1
fi
echo "VERIFICATION PASSED: symbol parity clean and all differential tests green"
echo "across every feature combination in both the debug and release profiles."
