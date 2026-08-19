#!/usr/bin/env bash
# Full verification sweep: every build-time feature combination x profile.
#
#   ./run_all_checks.sh
#
# Steps
#   1. enumerate the [features] combinations from Cargo.toml (there are none, so
#      the only combination is the empty one -> `--no-default-features`)
#   2. cargo check every combination (all targets)
#   3. build the C artifacts (cmake executable + shared library) from the
#      untouched c_src sources
#   4. diff `nm -D` between the C and the Rust shared object
#   5. cargo test every combination in the dev and release profiles

set -uo pipefail
cd "$(dirname "$0")"
CARGO_FLAGS="--offline"
fail=0
step() { printf '\n=== %s ===\n' "$*"; }
check() { if [ "$1" -ne 0 ]; then echo "FAILED: $2"; fail=1; else echo "ok: $2"; fi; }

step "1. feature combinations declared in Cargo.toml"
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "no [features] section -> exactly one combination: <none>"
  COMBOS=("")
else
  # power set of the declared features
  feats=($FEATURES)
  n=${#feats[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${feats[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
  printf 'features: %s\n' "$FEATURES"
fi
printf 'combination count: %d\n' "${#COMBOS[@]}"

step "2. cargo check for every combination"
for combo in "${COMBOS[@]}"; do
  label="--no-default-features --features '${combo}'"
  if [ -z "$combo" ]; then
    timeout 600 cargo check $CARGO_FLAGS --no-default-features --all-targets >/dev/null 2>&1
  else
    timeout 600 cargo check $CARGO_FLAGS --no-default-features --features "$combo" --all-targets >/dev/null 2>&1
  fi
  check $? "cargo check $label"
done
timeout 600 cargo check $CARGO_FLAGS --all-targets >/dev/null 2>&1
check $? "cargo check (default features)"

step "3. build the C artifacts"
(mkdir -p c_src/build && cd c_src/build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null &&
  gcc -shared -fPIC -o libcdriver.so ../src/main.c)
check $? "cmake executable + libcdriver.so"

step "4. build the Rust artifacts and diff nm -D"
timeout 600 cargo build $CARGO_FLAGS >/dev/null 2>&1
check $? "cargo build (dev)"
timeout 600 cargo build $CARGO_FLAGS --release >/dev/null 2>&1
check $? "cargo build (release)"
for prof in debug release; do
  csyms=$(nm -D --defined-only c_src/build/libcdriver.so | awk '{print $NF}' | sort -u)
  rsyms=$(nm -D --defined-only "target/$prof/libdriver.so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
  if [ -n "$missing" ]; then
    echo "FAILED: symbols exported by C but missing from Rust ($prof):"
    echo "$missing"
    fail=1
  else
    echo "ok: symbol parity ($prof) — $(echo "$csyms" | wc -l) C symbols all present"
  fi
done

step "5. cargo test for every combination x profile"
for combo in "${COMBOS[@]}"; do
  for prof in "" "--release"; do
    if [ -z "$combo" ]; then
      timeout 600 cargo test $CARGO_FLAGS --no-default-features $prof 2>&1 | tail -3
      rc=${PIPESTATUS[0]}
    else
      timeout 600 cargo test $CARGO_FLAGS --no-default-features --features "$combo" $prof 2>&1 | tail -3
      rc=${PIPESTATUS[0]}
    fi
    check $rc "cargo test --no-default-features --features '${combo}' ${prof:-(dev)}"
  done
done
timeout 600 cargo test $CARGO_FLAGS 2>&1 | tail -3
check ${PIPESTATUS[0]} "cargo test (default features, dev)"

printf '\n=== RESULT ===\n'
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit $fail
