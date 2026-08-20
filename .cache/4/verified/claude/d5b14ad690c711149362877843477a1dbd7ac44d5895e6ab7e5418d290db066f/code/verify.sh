#!/usr/bin/env bash
# Full verification sweep: every build configuration x every test phase.
#
# `Cargo.toml` declares no [features], so the complete set of feature
# combinations is {default} == {--no-default-features} == {--all-features};
# all three are still exercised so that adding a feature later cannot silently
# skip a configuration (tests/parity.rs::d05 asserts the claim itself).
set -u
cd "$(dirname "$0")"
fail=0

echo "=== building the reference C binary ==="
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

COMBOS=("" "--no-default-features" "--all-features")

log=$(mktemp)

for combo in "${COMBOS[@]}"; do
    label=${combo:-default}
    echo "=== cargo check ($label) ==="
    timeout 600 cargo check --offline $combo --all-targets > "$log" 2>&1 \
        || { echo "CHECK FAILED ($label)"; tail -20 "$log"; fail=1; }
    grep -E "^(warning|error)" "$log" | sort | uniq -c | head -5
done

for combo in "${COMBOS[@]}"; do
    label=${combo:-default}
    echo "=== cargo test ($label, debug binary) ==="
    timeout 600 cargo test --offline $combo > "$log" 2>&1 \
        || { echo "TESTS FAILED ($label)"; grep -E "FAILED|panicked" "$log" | head -10; fail=1; }
    grep -E "Running|result:" "$log"
done

echo "=== release profile (panic = abort) ==="
timeout 600 cargo build --offline --release >/dev/null 2>&1 || { echo "RELEASE BUILD FAILED"; fail=1; }
echo "=== cargo test (default, release binary under test) ==="
DRIVER_RUST_BIN="$PWD/target/release/driver" timeout 600 cargo test --offline > "$log" 2>&1 \
    || { echo "RELEASE TESTS FAILED"; grep -E "FAILED|panicked" "$log" | head -10; fail=1; }
grep -E "Running|result:" "$log"
rm -f "$log"

echo "=== nm -D symbol diff (C -> Rust) ==="
csym=$(mktemp); rsym=$(mktemp)
nm -D --defined-only c_src/build/driver | awk '{print $NF}' | sort > "$csym"
nm -D --defined-only target/release/driver | awk '{print $NF}' | sort > "$rsym"
if diff "$csym" "$rsym" > /dev/null; then
    echo "(identical)"
else
    echo "C-only defined dynamic symbols:"
    comm -23 "$csym" "$rsym" | sed 's/^/  /'
    echo "  -> glibc 'stdin' copy relocation only; see SYMBOLS.md"
    if comm -23 "$csym" "$rsym" | grep -qv "^stdin"; then
        echo "UNEXPLAINED MISSING SYMBOL"; fail=1
    fi
fi
rm -f "$csym" "$rsym"

if [ $fail -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $fail
