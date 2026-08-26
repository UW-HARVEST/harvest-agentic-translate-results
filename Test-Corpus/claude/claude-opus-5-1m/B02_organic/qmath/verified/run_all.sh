#!/bin/sh
# Runs the whole differential suite in EVERY build configuration.
#
# c_src/CMakeLists.txt has no options, so Cargo.toml has a single empty feature
# set: `--no-default-features`, the default and `--all-features` are the only
# invocations and they all select the same configuration.  All three are run
# anyway, plus a release build -- that is where Rust's debug-assertion pointer
# checks are off, so the NULL-dereference cases fault exactly like the C ones.
set -eu
cd "$(dirname "$0")"

LOG="${TMPDIR:-/tmp}/run_all.$$.log"
trap 'rm -f "$LOG"' EXIT

TARGETS="scalar vectors angles planes qshared data errors driver_cli nan_payloads"

run_suite() {
    label="$1"
    shift
    fail=0
    for t in $TARGETS; do
        if timeout 600 cargo test --offline "$@" --test "$t" > "$LOG" 2>&1; then
            printf '  %-13s %s\n' "$t" "$(grep -E '^test result' "$LOG" | head -1)"
        else
            printf '  %-13s FAILED\n' "$t"
            grep -E '^(test result|---- |thread)' "$LOG" | head -20
            fail=1
        fi
    done
    if [ "$fail" != 0 ]; then
        echo "*** $label FAILED"
        exit 1
    fi
    echo "  -> $label: all targets passed"
}

./build_c.sh > /dev/null
echo "=== C shared objects built ==="

for combo in "--no-default-features" "--features=" "--all-features"; do
    echo
    echo "=================================================================="
    echo "=== configuration: cargo ... $combo"
    echo "=================================================================="
    cargo build --offline "$combo" 2>&1 | grep -E "^(error|warning)" || true
    ./check_symbols.sh | sed 's/^/  /'
    run_suite "dev $combo" "$combo"
done

echo
echo "=================================================================="
echo "=== configuration: --release (rustc debug-assertions off)"
echo "=================================================================="
cargo build --offline --release 2>&1 | grep -E "^(error|warning)" || true
run_suite "release" --release

echo
echo "=================================================================="
echo "=== NaN-payload survey (DIFF_STRICT_NAN=1): informational"
echo "=================================================================="
for t in $TARGETS; do
    DIFF_STRICT_NAN=1 timeout 600 cargo test --offline --test "$t" > "$LOG" 2>&1 || true
    printf '  %-13s %s\n' "$t" "$(grep -E '^test result' "$LOG" | head -1)"
done
echo "  (failures here are the documented NaN-payload deviation, see NOTES.md)"

echo
echo "ALL CONFIGURATIONS PASSED"
