#!/usr/bin/env bash
# Differential verification driver.
#
#  1. enumerates every valid Cargo feature combination from Cargo.toml,
#  2. runs `cargo check` for each one,
#  3. builds the C shared library and the C executable from c_src/ (writing
#     only into target/, never into c_src/),
#  4. compares the exported symbols of the C and Rust shared libraries,
#  5. runs the full differential test suite for every feature combination.
#
# Usage: ./verify.sh
set -uo pipefail

cd "$(dirname "$0")"
fail=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   [ok] %s\n' "$*"; }
bad()  { printf '   [FAIL] %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "1. Enumerating feature combinations declared in Cargo.toml"
# Every feature name in the [features] table, if the table exists at all.
features=$(awk '
  /^\[features\]/ { inside = 1; next }
  /^\[/           { inside = 0 }
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
' Cargo.toml)

if [ -z "$features" ]; then
  echo "   no [features] table: the only valid combination is the empty set"
  combos=("")
else
  # Full power set of the declared features.
  feats=($features)
  n=${#feats[@]}
  combos=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${feats[$i]}"
      fi
    done
    combos+=("$combo")
  done
fi
echo "   ${#combos[@]} combination(s): $(for c in "${combos[@]}"; do printf '[%s] ' "${c:-<none>}"; done)"

# ---------------------------------------------------------------------------
step "2. cargo check for every combination (all targets)"
for combo in "${combos[@]}"; do
  label="${combo:-<none>}"
  if timeout 600 cargo check --offline --no-default-features \
       ${combo:+--features "$combo"} --all-targets >/dev/null 2>&1; then
    ok "cargo check --no-default-features --features $label"
  else
    bad "cargo check --no-default-features --features $label"
    timeout 600 cargo check --offline --no-default-features \
      ${combo:+--features "$combo"} --all-targets 2>&1 | tail -20
  fi
done

# ---------------------------------------------------------------------------
step "3. Building the C shared library and executable from c_src/"
mkdir -p target/cbuild
if gcc -shared -fPIC -O2 -o target/cbuild/libcdriver.so c_src/src/main.c 2>&1; then
  ok "target/cbuild/libcdriver.so"
else
  bad "C shared library"
fi
if cmake -S c_src -B target/cexe >/dev/null 2>&1 && \
   cmake --build target/cexe >/dev/null 2>&1; then
  ok "target/cexe/driver (via c_src/CMakeLists.txt)"
else
  bad "C executable"
fi
if timeout 600 cargo build --offline >/dev/null 2>&1; then
  ok "target/debug/libdriver.so and target/debug/driver"
else
  bad "Rust build"
fi

# ---------------------------------------------------------------------------
step "4. Symbol parity (nm -D --defined-only)"
syms() { nm -D --defined-only "$1" | awk '$2 == "T" || $2 == "W" { print $3 }' | sort -u; }
syms target/cbuild/libcdriver.so > target/c.syms
syms target/debug/libdriver.so   > target/rust.syms
printf '   C    exports: %s\n' "$(tr '\n' ' ' < target/c.syms)"
missing=$(comm -23 target/c.syms target/rust.syms)
if [ -z "$missing" ]; then
  ok "every C symbol is exported by the Rust .so (diff empty)"
else
  bad "missing from the Rust .so: $(echo "$missing" | tr '\n' ' ')"
fi

# ---------------------------------------------------------------------------
step "5. Differential test suite for every combination"
for combo in "${combos[@]}"; do
  label="${combo:-<none>}"
  echo "   --- features: $label"
  if timeout 600 cargo test --offline --no-default-features \
       ${combo:+--features "$combo"} 2>&1 | tee target/test-out.txt | \
       grep -E "^test result|FAILED" ; then
    if grep -q "FAILED\|error\[" target/test-out.txt; then
      bad "cargo test --features $label"
    else
      ok "cargo test --features $label"
    fi
  else
    bad "cargo test --features $label (no results)"
  fi
done

# ---------------------------------------------------------------------------
if [ "$fail" -eq 0 ]; then
  printf '\n\033[1;32mALL CHECKS PASSED\033[0m\n'
else
  printf '\n\033[1;31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$fail"
