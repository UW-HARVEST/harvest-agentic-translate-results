#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every build
# configuration.
#
#  1. build the C reference shared library
#  2. enumerate the crate's feature combinations from Cargo.toml
#  3. cargo check + build + test each combination
#  4. compare exported symbols of the two .so files
#
# Usage: ./verify.sh            (from the repository root or from translation/)
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$here/.." && pwd)
cargo_toml="$here/Cargo.toml"
fail=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- 1. C library
step "building C reference library"
mkdir -p "$root/c_src/build"
(
  cd "$root/c_src/build" &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/lz4_cmake.log 2>&1 &&
    timeout 600 cmake --build . >/tmp/lz4_cbuild.log 2>&1
) || {
  echo "C build FAILED; see /tmp/lz4_cmake.log and /tmp/lz4_cbuild.log"
  tail -n 20 /tmp/lz4_cbuild.log
  exit 1
}
c_so="$root/c_src/build/liblz4.so"
[[ -f $c_so ]] || { echo "missing $c_so"; exit 1; }
echo "ok: $c_so"

# ------------------------------------------------- 2. enumerate feature combos
# Every subset of the [features] table (excluding "default"), plus the plain
# default build. This crate currently declares no features, so the list is just
# the default configuration; the loop below stays correct if features are added.
mapfile -t features < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$cargo_toml"
)

combos=("<default>")
n=${#features[@]}
if ((n > 0)); then
  combos=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${features[i]}")
    done
    if ((${#sel[@]} == 0)); then
      combos+=("<none>")
    else
      combos+=("$(
        IFS=,
        echo "${sel[*]}"
      )")
    fi
  done
  combos+=("<default>")
fi

step "feature combinations to verify (${#combos[@]})"
printf '  %s\n' "${combos[@]}"

# --------------------------------------------------- 3. check / build / test
cd "$here"
for combo in "${combos[@]}"; do
  case "$combo" in
  "<default>") flags=() ;;
  "<none>") flags=(--no-default-features) ;;
  *) flags=(--no-default-features --features "$combo") ;;
  esac

  step "cargo check [$combo]"
  if ! timeout 600 cargo check "${flags[@]}" 2>&1 | tail -n 5; then
    echo "CHECK FAILED for $combo"
    fail=1
    continue
  fi

  step "cargo build --release [$combo]"
  if ! timeout 600 cargo build --release "${flags[@]}" 2>&1 | tail -n 5; then
    echo "BUILD FAILED for $combo"
    fail=1
    continue
  fi

  rust_so="$here/target/release/liblz4.so"
  [[ -f $rust_so ]] || { echo "missing $rust_so"; fail=1; continue; }

  step "symbol comparison [$combo]"
  nm -D --defined-only "$c_so" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort >/tmp/lz4_c_syms.txt
  nm -D --defined-only "$rust_so" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort >/tmp/lz4_r_syms.txt
  missing=$(comm -23 /tmp/lz4_c_syms.txt /tmp/lz4_r_syms.txt)
  extra=$(comm -13 /tmp/lz4_c_syms.txt /tmp/lz4_r_syms.txt)
  if [[ -n $missing ]]; then
    echo "MISSING in Rust .so:"
    echo "$missing"
    fail=1
  fi
  if [[ -n $extra ]]; then
    echo "EXTRA in Rust .so:"
    echo "$extra"
  fi
  [[ -z $missing ]] && echo "ok: $(wc -l </tmp/lz4_c_syms.txt) symbols exported by both"

  step "cargo test --release [$combo]"
  if ! timeout 600 cargo test --release "${flags[@]}" 2>&1 | grep -E "^(test |running|test result|error)" ; then
    echo "TEST FAILED for $combo"
    fail=1
  fi
done

step "summary"
if ((fail)); then
  echo "FAILURES DETECTED"
  exit 1
fi
echo "all configurations verified"
