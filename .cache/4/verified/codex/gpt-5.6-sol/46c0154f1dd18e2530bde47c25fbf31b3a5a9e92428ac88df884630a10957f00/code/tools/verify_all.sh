#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
mkdir -p logs c_src/build

timeout 600 cmake -S c_src -B c_src/build \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON > logs/verify-cmake-configure.log 2>&1
timeout 600 cmake --build c_src/build > logs/verify-cmake-build.log 2>&1

# Cargo.toml has no [features] table, so the sole valid combination is empty.
feature_combinations=("")
for features in "${feature_combinations[@]}"; do
  suffix="${features:-none}"
  cargo_args=(--no-default-features)
  if [[ -n "$features" ]]; then
    cargo_args+=(--features "$features")
  fi

  timeout 600 cargo check "${cargo_args[@]}" \
    > "logs/verify-check-${suffix}.log" 2>&1
  timeout 600 cargo build "${cargo_args[@]}" \
    > "logs/verify-build-${suffix}.log" 2>&1
  timeout 600 cargo test "${cargo_args[@]}" -- --test-threads=1 \
    > "logs/verify-test-${suffix}.log" 2>&1
done

nm -D --defined-only c_src/build/libpcre2.so |
  awk '{print $3}' | sort > logs/verify-c-symbol-names.txt
nm -D --defined-only target/debug/libpcre2.so |
  awk '{print $3}' | sort > logs/verify-rust-symbol-names.txt
comm -23 logs/verify-c-symbol-names.txt logs/verify-rust-symbol-names.txt \
  > logs/verify-missing-rust-symbols.txt
comm -13 logs/verify-c-symbol-names.txt logs/verify-rust-symbol-names.txt \
  > logs/verify-extra-rust-symbols.txt

test ! -s logs/verify-missing-rust-symbols.txt
test ! -s logs/verify-extra-rust-symbols.txt

printf 'verified %d feature combination(s); %d symbols; no symbol diff\n' \
  "${#feature_combinations[@]}" "$(wc -l < logs/verify-c-symbol-names.txt)"
