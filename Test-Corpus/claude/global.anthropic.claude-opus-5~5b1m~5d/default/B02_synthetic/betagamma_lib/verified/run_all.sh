#!/usr/bin/env bash
# Phase D driver: verify every build profile x every cargo feature combination.
#
#   ./run_all.sh
#
# For each configuration it (1) builds the Rust cdylib, (2) diffs `nm -D`
# against the C .so and fails on any missing symbol, (3) runs the full
# differential + error-path + symbol test suites against that exact .so.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
cd "$here"

fail=0
log_dir="$here/target/verify-logs"
mkdir -p "$log_dir"

# --------------------------------------------------------------------------
# 1. Build the C shared library (ground truth).
# --------------------------------------------------------------------------
echo "== building C .so =="
( cd "$root/c_src" \
  && mkdir -p build \
  && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >"$log_dir/c-build.log" 2>&1 \
  || { echo "FAIL: C build"; tail -20 "$log_dir/c-build.log"; exit 1; }

c_so="$(ls "$root"/c_src/build/lib*.so | head -1)"
echo "   C .so: $c_so"

# --------------------------------------------------------------------------
# 2. Enumerate cargo feature combinations from Cargo.toml.
#    (This crate declares no [features], so the set is: default only.
#    The loop is written generically so it keeps working if features are added.)
# --------------------------------------------------------------------------
mapfile -t features < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

combos=()
combos+=("default:")                       # default features
combos+=("nodefault:--no-default-features") # explicitly none
if [ "${#features[@]}" -gt 0 ]; then
  for f in "${features[@]}"; do
    combos+=("$f:--no-default-features --features $f")
  done
  # all features at once
  all=$(IFS=, ; echo "${features[*]}")
  combos+=("allfeatures:--no-default-features --features $all")
  combos+=("allfeatures-plus-default:--all-features")
fi
echo "== feature combinations: ${#combos[@]} (declared features: ${#features[@]}) =="

# --------------------------------------------------------------------------
# 3. profile x feature-combo matrix
# --------------------------------------------------------------------------
nm_c="$log_dir/c.syms"
nm -D --defined-only "$c_so" | awk '{print $NF}' | sort -u >"$nm_c"

for profile in debug release; do
  if [ "$profile" = release ]; then prof_flag="--release"; else prof_flag=""; fi
  for combo in "${combos[@]}"; do
    name="${combo%%:*}"
    flags="${combo#*:}"
    tag="$profile-$name"
    echo
    echo "=================================================================="
    echo "== $tag   (cargo $prof_flag $flags)"
    echo "=================================================================="

    # (1) build the cdylib
    # shellcheck disable=SC2086
    if ! cargo build $prof_flag $flags >"$log_dir/$tag-build.log" 2>&1; then
      echo "FAIL[$tag]: cargo build"; tail -25 "$log_dir/$tag-build.log"; fail=1; continue
    fi
    rust_so="$here/target/$profile/libbetagamma_lib.so"
    if [ ! -f "$rust_so" ]; then
      echo "FAIL[$tag]: $rust_so not produced"; fail=1; continue
    fi

    # (2) symbol diff must be empty
    nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u >"$log_dir/$tag.syms"
    missing="$(comm -23 "$nm_c" "$log_dir/$tag.syms")"
    if [ -n "$missing" ]; then
      echo "FAIL[$tag]: symbols exported by C but MISSING from Rust:"
      echo "$missing" | sed 's/^/    /'
      fail=1
    else
      echo "   symbol diff: EMPTY ($(wc -l <"$nm_c") C symbols, all present in Rust)"
    fi

    # (3) run every test suite against this exact .so
    # shellcheck disable=SC2086
    if BETAGAMMA_RUST_SO="$rust_so" \
       timeout 550 cargo test $prof_flag $flags --tests -- --test-threads=1 \
       >"$log_dir/$tag-test.log" 2>&1; then
      grep -E "^test result:" "$log_dir/$tag-test.log" | sed 's/^/   /'
    else
      echo "FAIL[$tag]: cargo test"
      grep -E "^test result:|FAILED|panicked|^error" "$log_dir/$tag-test.log" | head -40 | sed 's/^/    /'
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED (logs in $log_dir)"
fi
exit "$fail"
