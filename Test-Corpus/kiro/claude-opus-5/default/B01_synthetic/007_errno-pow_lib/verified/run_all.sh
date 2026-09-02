#!/usr/bin/env bash
# Full verification pipeline: builds the C .so and the Rust cdylib, checks
# dynamic-symbol parity, and runs the differential suite under every feature
# combination and both profiles.
set -uo pipefail

CRATE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$CRATE/.." && pwd)"
rc=0
step() { echo; echo "=== $* ==="; }
check() { if [ "$1" -eq 0 ]; then echo "PASS  $2"; else echo "FAIL  $2"; rc=1; fi; }

step "Build C shared library"
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
check $? "C libpow.so built"
C_SO="$ROOT/c_src/build/libpow.so"

step "Enumerate feature combinations"
# Every feature declared in [features], as a power set. The crate declares
# none, so this yields exactly one (empty) combination.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' "$CRATE/Cargo.toml")
echo "declared features: ${FEATURES:-<none>}"
COMBOS=("")
for f in $FEATURES; do
  new=()
  for c in "${COMBOS[@]}"; do new+=("$c" "${c:+$c,}$f"); done
  COMBOS=("${new[@]}")
done
echo "combinations to verify: ${#COMBOS[@]} (plus the default-features build)"

for profile in debug release; do
  FLAG=""; [ "$profile" = release ] && FLAG="--release"
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-none}]"

    step "$label — cargo build"
    (cd "$CRATE" && timeout 600 cargo build $FLAG --no-default-features \
      ${combo:+--features "$combo"} --quiet)
    check $? "$label build"

    RUST_SO="$CRATE/target/$profile/libpow.so"
    step "$label — nm -D symbol parity"
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u))
    if [ -n "$missing" ]; then
      echo "symbols exported by C but MISSING from Rust:"; echo "$missing"
      check 1 "$label symbol parity"
    else
      check 0 "$label symbol parity (0 missing)"
    fi

    step "$label — differential suite against $profile cdylib"
    POW_RUST_SO="$RUST_SO" timeout 600 env -u CARGO_TARGET_DIR \
      cargo test --manifest-path "$CRATE/Cargo.toml" --quiet 2>&1 | tail -20
    check ${PIPESTATUS[0]} "$label differential suite"
  done
done

step "Mutation check (suite must detect injected bugs)"
timeout 600 "$CRATE/mutation_check.sh" | tail -3
check $? "mutation check"

step "Summary"
[ "$rc" -eq 0 ] && echo "ALL CHECKS PASSED" || echo "THERE WERE FAILURES"
exit "$rc"
