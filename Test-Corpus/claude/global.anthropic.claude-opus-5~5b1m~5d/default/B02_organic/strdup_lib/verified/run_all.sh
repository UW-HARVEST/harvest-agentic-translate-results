#!/usr/bin/env bash
# Full verification run: builds the C .so and the Rust .so, then runs every
# differential test under every feature combination and both profiles.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"

echo "=== 1. build the C shared library ==="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
c_so="$root/c_src/build/libdriver.so"
echo "    $c_so"

echo
echo "=== 2. enumerate cargo feature combinations ==="
# Features are read from Cargo.toml; with no [features] table the only
# combination is the default one.
feats=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' "$here/Cargo.toml")
if [ -z "$feats" ]; then
  echo "    no [features] table -> single configuration"
  combos=("" "--no-default-features" "--all-features")
else
  combos=("" "--no-default-features" "--all-features")
  for f in $feats; do combos+=("--no-default-features --features $f"); done
fi

echo
echo "=== 3. symbol parity ==="
for profile in debug release; do
  flag=""; [ "$profile" = release ] && flag="--release"
  ( cd "$here" && cargo build --offline $flag >/dev/null 2>&1 )
  rs_so="$here/target/$profile/libdriver.so"
  diff <(nm -D --defined-only "$c_so"  | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort) \
       <(nm -D --defined-only "$rs_so" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort) \
    && echo "    $profile: symbol diff EMPTY (ok)" \
    || { echo "    $profile: SYMBOL DIFF NOT EMPTY"; exit 1; }
done

echo
echo "=== 4. differential tests, all profiles x all feature combos ==="
for profile in debug release; do
  flag=""; [ "$profile" = release ] && flag="--release"
  for combo in "${combos[@]}"; do
    echo "--- profile=$profile combo='${combo:-<default>}' ---"
    ( cd "$here" && timeout 600 cargo test --offline $flag $combo 2>&1 \
        | grep -E '^(test result|error|warning: unused)' )
  done
done

echo
echo "=== ALL CONFIGURATIONS PASSED ==="
