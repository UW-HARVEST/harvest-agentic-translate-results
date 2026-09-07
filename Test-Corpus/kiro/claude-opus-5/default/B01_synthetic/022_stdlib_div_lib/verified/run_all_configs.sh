#!/usr/bin/env bash
# Build the C reference .so and the Rust .so, then run the full differential
# suite under every build configuration.
#
# `cargo test` does NOT build a `crate-type = ["cdylib"]` library target (the
# integration tests do not link it), so the .so must be built explicitly first.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
FAILED=0

echo "=== building the C reference shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }

# Feature combinations, derived from Cargo.toml rather than hardcoded.
FEATURE_LIST=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' "$HERE/Cargo.toml")
COMBOS=("default")
for f in $FEATURE_LIST; do COMBOS+=("--no-default-features --features $f"); done
if [ -n "$FEATURE_LIST" ]; then COMBOS+=("--no-default-features"); else COMBOS+=("--no-default-features"); fi

for PROFILE in debug release; do
  for COMBO in "${COMBOS[@]}"; do
    if [ "$COMBO" = "default" ]; then FLAGS=(); else read -r -a FLAGS <<<"$COMBO"; fi
    if [ "$PROFILE" = "release" ]; then FLAGS+=(--release); fi

    echo
    echo "=== profile=$PROFILE features=[$COMBO] ==="

    if ! timeout 600 cargo build "${FLAGS[@]}" >/dev/null 2>&1; then
      echo "  cargo build FAILED"; FAILED=1; continue
    fi
    RUST_SO="$HERE/target/$PROFILE/libdriver.so"
    if [ ! -f "$RUST_SO" ]; then echo "  missing $RUST_SO"; FAILED=1; continue; fi

    echo "  --- nm -D symbol parity ---"
    DIFF=$(comm -3 \
      <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort))
    if [ -n "$DIFF" ]; then
      echo "  SYMBOL DIFF NOT EMPTY:"; echo "$DIFF" | sed 's/^/    /'; FAILED=1
    else
      echo "  symbol diff empty ($(nm -D --defined-only "$C_SO" | wc -l) exported symbol(s))"
    fi

    # The .so under test is pinned explicitly so the release run cannot
    # accidentally exercise the debug artifact. --no-fail-fast so that a
    # failure in one test binary does not hide the other's results.
    C_DRIVER_SO="$C_SO" RUST_DRIVER_SO="$RUST_SO" \
      timeout 600 cargo test --no-fail-fast "${FLAGS[@]}" -- --test-threads=1 2>&1 \
      | grep -E "^(test result|running|\s*Running|test .* FAILED|error)" 
    # shellcheck disable=SC2181
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then echo "  TESTS FAILED"; FAILED=1; fi
  done
done

echo
if [ "$FAILED" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$FAILED"
