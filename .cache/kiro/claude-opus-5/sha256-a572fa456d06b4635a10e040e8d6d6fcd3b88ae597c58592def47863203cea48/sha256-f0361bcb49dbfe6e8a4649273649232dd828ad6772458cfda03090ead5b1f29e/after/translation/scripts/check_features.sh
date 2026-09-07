#!/usr/bin/env bash
# Phase D driver: build the C .so, enumerate every cargo feature combination
# from Cargo.toml, and run the full differential suite (Phases B + C) for each
# combination, against both the debug and the release Rust cdylib.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root="$(cd "$here/.." && pwd)"
cd "$here"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
# 1. Build the C shared library
# --------------------------------------------------------------------------
note "building C .so"
( cd "$root/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/ima_cmake.log 2>&1 \
    && cmake --build . >>/tmp/ima_cmake.log 2>&1 ) \
  || { echo "C build FAILED, see /tmp/ima_cmake.log"; tail -20 /tmp/ima_cmake.log; exit 1; }
C_SO=$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C .so: $C_SO"

# --------------------------------------------------------------------------
# 2. Enumerate feature combinations (power set of [features] in Cargo.toml)
# --------------------------------------------------------------------------
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
echo "features found: ${#FEATURES[@]} (${FEATURES[*]-none})"

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  COMBOS=("__default__")           # no [features] section -> only the default build
else
  COMBOS+=("__default__")
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("--no-default-features${combo:+ --features $combo}")
  done
fi

# --------------------------------------------------------------------------
# 3. cargo check + cargo test for every combination x profile,
#    against both the debug and the release cdylib.
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "__default__" ]]; then flags=(); label="(default features)";
  else read -r -a flags <<< "$combo"; label="$combo"; fi

  note "cargo check $label"
  timeout 300 cargo check --all-targets "${flags[@]}" 2>&1 | tail -3 || fail=1

  for profile in dev release; do
    prof_flags=(); prof_dir=debug
    if [[ $profile == release ]]; then prof_flags=(--release); prof_dir=release; fi

    note "cargo build $label profile=$profile"
    timeout 600 cargo build "${prof_flags[@]}" "${flags[@]}" 2>&1 | tail -3 || fail=1

    note "nm -D symbol diff  C vs Rust ($label profile=$profile)"
    R_SO="$here/target/$prof_dir/libima_parse_lib.so"
    diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
         <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort) \
      && echo "symbol diff EMPTY -- OK" || { echo "SYMBOL DIFF NON-EMPTY"; fail=1; }

    # Run the suite against BOTH cdylib profiles, so the shipped (release)
    # library is differential-tested too.
    for so_dir in debug release; do
      so="$here/target/$so_dir/libima_parse_lib.so"
      [[ -f "$so" ]] || continue
      note "cargo test $label test-profile=$profile rust-so=$so_dir"
      IMA_C_SO="$C_SO" IMA_RUST_SO="$so" \
        timeout 600 cargo test "${prof_flags[@]}" "${flags[@]}" 2>&1 \
        | grep -E 'test result|panicked|FAILED|error\[|^error' \
        || true
      IMA_C_SO="$C_SO" IMA_RUST_SO="$so" \
        timeout 600 cargo test "${prof_flags[@]}" "${flags[@]}" >/tmp/ima_test.log 2>&1 \
        || { echo "TESTS FAILED ($label profile=$profile so=$so_dir)"; tail -40 /tmp/ima_test.log; fail=1; }
    done
  done
done

note "RESULT"
if (( fail )); then echo "FAIL"; exit 1; else echo "ALL COMBINATIONS PASS"; fi
