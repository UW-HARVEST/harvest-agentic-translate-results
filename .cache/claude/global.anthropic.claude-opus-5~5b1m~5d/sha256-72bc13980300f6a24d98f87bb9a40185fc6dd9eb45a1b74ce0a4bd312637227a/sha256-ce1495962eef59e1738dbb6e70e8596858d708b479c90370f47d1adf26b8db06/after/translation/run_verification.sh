#!/usr/bin/env bash
# Full differential verification driver.
#
#   ./run_verification.sh
#
# 1. builds the C shared library
# 2. builds the Rust cdylib
# 3. diffs `nm -D` between the two (Phase A / D symbol-parity gate)
# 4. runs every integration test under EVERY cargo feature combination
#
# There is no network in this environment, so every cargo invocation uses
# --offline (the crates are already in ~/.cargo/registry).
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
CSO="$ROOT/c_src/build/libpcre2.so"
RSO="$HERE/target/release/libpcre2.so"
FAIL=0

say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- 1. build C
say "building C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
  && cmake --build . -- -j"$(nproc)" > /dev/null ) || { echo "C BUILD FAILED"; exit 1; }
ls -l "$CSO"

# ------------------------------------------------------------- 2. build Rust
say "building Rust cdylib"
( cd "$HERE" && cargo build --offline --release ) || { echo "RUST BUILD FAILED"; exit 1; }
ls -l "$RSO"

# --------------------------------------------------------- 3. symbol parity
say "symbol parity (nm -D)"
nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u > "${TMPDIR:-/tmp}/.pcre2_c_syms"
nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u > "${TMPDIR:-/tmp}/.pcre2_r_syms"
MISSING=$(comm -23 "${TMPDIR:-/tmp}/.pcre2_c_syms" "${TMPDIR:-/tmp}/.pcre2_r_syms")
printf 'C exports  : %s\n' "$(wc -l < "${TMPDIR:-/tmp}/.pcre2_c_syms")"
printf 'Rust exports: %s\n' "$(wc -l < "${TMPDIR:-/tmp}/.pcre2_r_syms")"
if [ -n "$MISSING" ]; then
  echo "MISSING FROM RUST:"; echo "$MISSING"; FAIL=1
else
  echo "symbol diff is EMPTY"
fi
# undefined, non-libc symbols in the Rust object
BADUNDEF=$(nm -D --undefined-only "$RSO" | awk '{print $2}' | grep -E '^_?pcre2_' || true)
if [ -n "$BADUNDEF" ]; then
  echo "UNDEFINED pcre2 SYMBOLS IN RUST .so:"; echo "$BADUNDEF"; FAIL=1
else
  echo "no undefined non-libc symbols"
fi

# ------------------------------------------------- 4. every feature combination
# Enumerate the features declared in Cargo.toml and build the power set. This
# crate declares none, so the loop below degenerates to the two equivalent
# no-feature configurations -- but it is written generically so that adding a
# feature automatically widens the verification.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' "$HERE/Cargo.toml"
)
COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("--no-default-features")
  COMBOS+=("")
else
  N=${#FEATURES[@]}
  for ((mask=0; mask<(1<<N); mask++)); do
    sel=()
    for ((i=0; i<N; i++)); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    if [ "${#sel[@]}" -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  COMBOS+=("")   # the default feature set
fi

for combo in "${COMBOS[@]}"; do
  say "cargo test --offline ${combo:-<default features>}"
  # shellcheck disable=SC2086
  ( cd "$HERE" && cargo build --offline --release $combo ) || { FAIL=1; continue; }
  # shellcheck disable=SC2086
  ( cd "$HERE" && cargo test --offline $combo -- --test-threads="$(nproc)" ) || FAIL=1
done

say "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
