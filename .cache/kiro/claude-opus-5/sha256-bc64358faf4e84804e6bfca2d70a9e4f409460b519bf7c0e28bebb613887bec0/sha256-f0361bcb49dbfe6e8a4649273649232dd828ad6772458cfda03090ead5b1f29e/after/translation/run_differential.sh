#!/usr/bin/env bash
# Rebuild BOTH shared libraries, then run the differential suite across every
# feature combination.
#
# `cargo test` does not rebuild a `cdylib` target, so the explicit
# `cargo build` below is mandatory — without it the suite would load a stale
# `.so`. The suite also asserts artifact freshness itself as a backstop.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

echo "=== building C shared library ==="
mkdir -p "$root/c_src/build"
cmake -S "$root/c_src" -B "$root/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build "$root/c_src/build" >/dev/null
echo "ok: $root/c_src/build/libdriver.so"

# Enumerate feature combinations from Cargo.toml. This crate declares no
# [features] table, so the set is just the default configuration; the loop is
# written generically so it stays correct if features are ever added.
mapfile -t features < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,""); gsub(/ /,""); if ($0 != "default") print}' \
    "$here/Cargo.toml"
)

declare -a combos=("--no-default-features" "")   # "" == default features
if [ "${#features[@]}" -gt 0 ]; then
  combos+=("--all-features")
  for f in "${features[@]}"; do
    combos+=("--no-default-features --features $f")
  done
fi

status=0
for combo in "${combos[@]}"; do
  label="${combo:-<default features>}"
  echo
  echo "=== combo: $label ==="
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $combo >/dev/null
  # shellcheck disable=SC2086
  if timeout 600 cargo test --release $combo -- --test-threads=4; then
    echo "PASS  $label"
  else
    echo "FAIL  $label"
    status=1
  fi
done

echo
if [ "$status" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$status"
