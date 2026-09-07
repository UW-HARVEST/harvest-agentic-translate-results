#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under every configuration.
#
#   ./run_all.sh
#
# 1. (re)builds the C reference .so with CMake
# 2. enumerates the cargo feature combinations from Cargo.toml
# 3. for each (feature-combo x profile): builds the Rust cdylib, runs the suite
# 4. diffs `nm -D` between the two .so files
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
cd "$here"

CARGO_OFFLINE=${CARGO_OFFLINE:---offline}
fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad()  { printf '\033[31mFAIL: %s\033[0m\n' "$*"; fail=1; }

# ---------------------------------------------------------------- C reference
note "building the C reference library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
c_so="$(ls -1 "$root"/c_src/build/lib*.so | head -n1)"
echo "  C .so: $c_so"

# ------------------------------------------------------- feature combinations
# Cargo.toml declares no [features] table, so the combination set is just the
# default set and the empty set. Derived, not assumed:
declared="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,"");print}' Cargo.toml)"
if [ -n "$declared" ]; then
  echo "  declared features: $declared"
  combos=("" "--no-default-features")
  for f in $declared; do
    combos+=("--no-default-features --features $f")
  done
  combos+=("--all-features")
else
  echo "  no [features] table in Cargo.toml"
  combos=("" "--no-default-features" "--all-features")
fi

# ------------------------------------------------------------------- run matrix
for profile in "" "--release"; do
  for combo in "${combos[@]}"; do
    label="profile='${profile:-dev}' features='${combo:-default}'"
    note "$label"

    # shellcheck disable=SC2086
    if ! cargo build $CARGO_OFFLINE $profile $combo >/dev/null 2>&1; then
      bad "cargo build ($label)"; continue
    fi

    prof_dir="target/$([ -n "$profile" ] && echo release || echo debug)"
    rust_so="$prof_dir/libto_barycentric_lib.so"
    [ -f "$rust_so" ] || { bad "no cdylib at $rust_so ($label)"; continue; }

    # symbol diff for this exact build
    if ! diff <(nm -D --defined-only "$c_so"    | awk 'NF>=2{print $NF}' | grep -v '^_' | sort -u) \
              <(nm -D --defined-only "$rust_so" | awk 'NF>=2{print $NF}' | grep -v '^_' | sort -u) \
         > "$prof_dir/symbol.diff"; then
      # only C-side lines ("<") are real failures
      if grep -q '^<' "$prof_dir/symbol.diff"; then
        bad "symbols missing from the Rust .so ($label)"; cat "$prof_dir/symbol.diff"
      fi
    fi

    # shellcheck disable=SC2086
    if ! C_LIB_PATH="$c_so" RUST_LIB_PATH="$(pwd)/$rust_so" \
         timeout 600 cargo test $CARGO_OFFLINE $profile $combo -- --test-threads=4 \
         > "$prof_dir/test.log" 2>&1; then
      bad "cargo test ($label)"; tail -n 40 "$prof_dir/test.log"
    else
      grep -h 'test result:' "$prof_dir/test.log" | sed 's/^/  /'
    fi
  done
done

# ------------------------------------------------------------------- binaries
note "binary (driver) parity"
if grep -q 'add_executable' "$root/c_src/CMakeLists.txt" 2>/dev/null \
   || [ -f src/main.rs ] || grep -q '^\[\[bin\]\]' Cargo.toml; then
  bad "a driver binary exists but this script does not compare it"
else
  echo "  n/a: neither project builds an executable (library only)"
fi

note "summary"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
