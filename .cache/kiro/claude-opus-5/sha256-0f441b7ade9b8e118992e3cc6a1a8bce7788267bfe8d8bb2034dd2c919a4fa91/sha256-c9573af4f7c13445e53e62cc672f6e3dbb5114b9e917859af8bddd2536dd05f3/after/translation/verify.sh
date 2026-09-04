#!/usr/bin/env bash
# Full verification matrix: builds the C reference, then for EVERY cargo feature
# combination rebuilds the Rust cdylib, diffs the exported symbols against the C
# .so, and runs the differential test suites.
#
#   usage: ./verify.sh            # symbol parity + all differential tests
#          ./verify.sh --quick    # skip the slowest CONFIGS rows
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
CSO="$ROOT/c_src/build/liblong.so"

QUICK=0
[[ "${1:-}" == "--quick" ]] && QUICK=1

# Every combination of the features declared in Cargo.toml.  There are no
# default features, so combo 1 is the default build.
mapfile -t FEATURES < <(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[a-zA-Z0-9_-]+ *=/{print $1}' "$HERE/Cargo.toml")
COMBOS=("")
for f in "${FEATURES[@]}"; do
  new=()
  for c in "${COMBOS[@]}"; do
    new+=("$c")
    if [[ -z "$c" ]]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS=("${new[@]}")
done

echo "### features declared: ${FEATURES[*]:-<none>}"
echo "### combinations to verify: ${#COMBOS[@]}"

if [[ ! -f "$CSO" ]]; then
  echo "### building C reference"
  (cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || exit 1
fi

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "=============================================================="
  echo "### feature combination: $label"
  echo "=============================================================="

  args=(--release --no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  (cd "$HERE" && cargo build "${args[@]}") || { fail=1; continue; }
  RSO="$HERE/target/release/liblong.so"

  echo "--- symbol parity (nm -D, defined symbols) ---"
  diff <(nm -D --defined-only "$CSO" | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$RSO" | awk '{print $NF}' | sort)
  if [[ $? -ne 0 ]]; then echo "SYMBOL DIFF NOT EMPTY"; fail=1; else echo "symbol diff empty: OK"; fi

  echo "--- unresolved non-libc symbols ---"
  if ldd -r "$RSO" 2>&1 | grep -q 'undefined symbol'; then
    ldd -r "$RSO" 2>&1 | grep 'undefined symbol'; fail=1
  else
    echo "none: OK"
  fi

  echo "--- differential tests ---"
  targs=(test "${args[@]}")
  if [[ $QUICK -eq 1 ]]; then
    (cd "$HERE" && cargo "${targs[@]}" -- --test-threads=1 \
        --skip row01 --skip row02 --skip row16 --skip row17 --skip row18 --skip row19 --skip row39) \
      || fail=1
  else
    (cd "$HERE" && cargo "${targs[@]}" --test errors -- --test-threads=1) || fail=1
    (cd "$HERE" && cargo "${targs[@]}" --test smoke  -- --test-threads=1) || fail=1
    (cd "$HERE" && cargo "${targs[@]}" --lib         -- --test-threads=1) || fail=1
    (cd "$HERE" && cargo "${targs[@]}" --test configs -- --test-threads=1 long_exec) || fail=1
    (cd "$HERE" && cargo "${targs[@]}" --test configs -- --test-threads=1 --skip long_exec) || fail=1
  fi
done

echo
if [[ $fail -eq 0 ]]; then echo "### ALL FEATURE COMBINATIONS PASSED"; else echo "### FAILURES PRESENT"; fi
exit $fail
