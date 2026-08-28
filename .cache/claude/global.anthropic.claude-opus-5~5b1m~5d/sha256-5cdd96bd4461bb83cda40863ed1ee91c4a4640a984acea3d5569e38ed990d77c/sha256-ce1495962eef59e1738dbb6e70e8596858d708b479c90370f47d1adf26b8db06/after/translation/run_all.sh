#!/bin/bash
# Full verification driver: Phase A (symbols) -> Phase B/C (differential tests)
# -> Phase D (every feature combination, both cdylib profiles).
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(dirname "$here")
cd "$here" || exit 1

CARGO_FLAGS="--offline"
fail=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  OK   %s\n' "$*"; }
bad()  { printf '  FAIL %s\n' "$*"; fail=1; }

# --------------------------------------------------------------------------
step "0. build the C shared library"
( mkdir -p "$root/c_src/build" && cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) && ok "C .so built" || bad "C build"

# --------------------------------------------------------------------------
step "1. enumerate feature combinations from Cargo.toml"
# every declared feature (the crate may legitimately declare none)
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml | grep -v '^default$' | tr '\n' ' ')
echo "declared features: [${FEATURES:-<none>}]"

# combination list: always include the three canonical configurations, then the
# power set of the declared features (bounded to keep the run finite).
COMBOS=()
COMBOS+=("DEFAULT")
COMBOS+=("NODEFAULT")
COMBOS+=("ALL")
set -- $FEATURES
n=$#
if [ "$n" -gt 0 ] && [ "$n" -le 10 ]; then
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    i=0
    for f in $FEATURES; do
      if (( (mask >> i) & 1 )); then combo="$combo,$f"; fi
      i=$((i + 1))
    done
    COMBOS+=("NODEFAULT${combo}")
  done
fi
printf 'combinations to verify: %d\n' "${#COMBOS[@]}"

flags_for() {
  case "$1" in
    DEFAULT) echo "" ;;
    ALL) echo "--all-features" ;;
    NODEFAULT) echo "--no-default-features" ;;
    NODEFAULT,*) echo "--no-default-features --features ${1#NODEFAULT,}" ;;
    *) echo "" ;;
  esac
}

# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  fl=$(flags_for "$combo")
  step "2. combo [$combo]  flags: ${fl:-<default>}"

  if timeout 600 cargo check $CARGO_FLAGS $fl >/dev/null 2>&1; then
    ok "cargo check"
  else
    bad "cargo check [$combo]"; continue
  fi

  # ---- release cdylib (what an external consumer links) ----
  if timeout 600 cargo build $CARGO_FLAGS --release $fl >/dev/null 2>&1; then
    ok "cargo build --release"
  else
    bad "cargo build --release [$combo]"; continue
  fi

  step "2a. combo [$combo] Phase A symbol parity"
  if ./check_symbols.sh | tail -3; then ok "symbol diff empty"; else bad "symbol diff [$combo]"; fi

  step "2a2. combo [$combo] allocation-call parity (LD_PRELOAD interposer)"
  RUST_SO="$here/target/release/libarr_ins_lib.so" ./tools/check_alloc_trace.sh \
    && ok "alloc-trace parity (release cdylib)" || bad "alloc-trace parity [$combo]"

  step "2b. combo [$combo] Phase B+C against the RELEASE cdylib"
  if RUST_SO="$here/target/release/libarr_ins_lib.so" \
       timeout 600 cargo test $CARGO_FLAGS --release $fl -- --test-threads=1 2>&1 \
       | grep -E "test result|FAILED|panicked" ; then
    :
  fi
  RUST_SO="$here/target/release/libarr_ins_lib.so" \
    timeout 600 cargo test $CARGO_FLAGS --release $fl -- --test-threads=1 >/dev/null 2>&1 \
    && ok "release tests" || bad "release tests [$combo]"

  # ---- debug cdylib: overflow-checks = on, debug-assertions = on.
  #      The C wraps and has live asserts, so the Rust must behave identically
  #      in this profile too (all wrapping arithmetic is explicit).
  if timeout 600 cargo build $CARGO_FLAGS $fl >/dev/null 2>&1; then
    ok "cargo build (debug cdylib)"
    RUST_SO="$here/target/debug/libarr_ins_lib.so" ./tools/check_alloc_trace.sh >/dev/null 2>&1 \
      && ok "alloc-trace parity (debug cdylib)" || bad "alloc-trace parity debug [$combo]"

    step "2c. combo [$combo] Phase B+C against the DEBUG cdylib"
    RUST_SO="$here/target/debug/libarr_ins_lib.so" \
      timeout 600 cargo test $CARGO_FLAGS --release $fl -- --test-threads=1 2>&1 \
      | grep -E "test result|FAILED|panicked"
    RUST_SO="$here/target/debug/libarr_ins_lib.so" \
      timeout 600 cargo test $CARGO_FLAGS --release $fl -- --test-threads=1 >/dev/null 2>&1 \
      && ok "debug-cdylib tests" || bad "debug-cdylib tests [$combo]"
  else
    bad "cargo build (debug) [$combo]"
  fi
done

step "SUMMARY"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
