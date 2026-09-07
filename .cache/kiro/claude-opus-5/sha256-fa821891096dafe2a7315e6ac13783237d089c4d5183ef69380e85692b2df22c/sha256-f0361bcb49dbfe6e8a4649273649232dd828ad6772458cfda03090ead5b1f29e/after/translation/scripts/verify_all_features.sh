#!/usr/bin/env bash
# Runs the full differential suite across every Cargo feature combination.
#
# Enumerates the features declared in Cargo.toml, builds the power set, and for
# each combination rebuilds the cdylib and re-runs all phases (B, C, D) against
# the freshly built .so. Also re-verifies the nm symbol diff per combination.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1   # translation/
ROOT="$(cd .. && pwd)"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"

if [[ -z "$C_SO" ]]; then
    echo "FATAL: C .so not built. Run:"
    echo "  cd c_src && mkdir -p build && cd build &&" \
         "cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi
echo "C .so: $C_SO"

# --- enumerate declared features -------------------------------------------
mapfile -t FEATURES < <(
    awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        sub(/[[:space:]]*=.*/,""); if ($0 != "default") print }' Cargo.toml
)
echo "declared features: ${FEATURES[*]:-(none)}"

# --- build the power set of feature combinations ----------------------------
COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
    # No [features] section: default and --no-default-features are the only
    # (identical) configurations. Verify both explicitly anyway.
    COMBOS=("default" "no-default")
else
    COMBOS+=("default" "no-default")
    for (( mask=1; mask < (1<<n); mask++ )); do
        combo=""
        for (( i=0; i<n; i++ )); do
            if (( mask & (1<<i) )); then
                combo="${combo:+$combo,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("no-default:$combo")
        COMBOS+=("all-default:$combo")
    done
fi
echo "combinations to verify: ${#COMBOS[@]}"

FAILED=()
for combo in "${COMBOS[@]}"; do
    case "$combo" in
        default)          ARGS=() ;;
        no-default)       ARGS=(--no-default-features) ;;
        no-default:*)     ARGS=(--no-default-features --features "${combo#no-default:}") ;;
        all-default:*)    ARGS=(--features "${combo#all-default:}") ;;
    esac

    echo
    echo "==================================================================="
    echo "CONFIG: $combo   (cargo ${ARGS[*]:-<default>})"
    echo "==================================================================="

    if ! timeout 600 cargo build --release "${ARGS[@]}" > /tmp/fc_build.log 2>&1; then
        echo "  BUILD FAILED"; tail -20 /tmp/fc_build.log; FAILED+=("$combo:build"); continue
    fi

    RUST_SO="$(find target/release -maxdepth 1 -name 'libmatrixsum_lib.so' | head -1)"
    echo "  rust .so: $RUST_SO"

    # symbol diff must be empty for this configuration
    diff <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort) \
         > /tmp/fc_symdiff.txt
    if grep -q '^<' /tmp/fc_symdiff.txt; then
        echo "  SYMBOL DIFF NOT EMPTY (missing from Rust):"
        grep '^<' /tmp/fc_symdiff.txt
        FAILED+=("$combo:symbols")
    else
        echo "  symbol diff: empty (0 missing)"
    fi

    if timeout 600 cargo test --release "${ARGS[@]}" > /tmp/fc_test.log 2>&1; then
        grep -h '^test result:' /tmp/fc_test.log | sed 's/^/  /'
    else
        echo "  TESTS FAILED"
        grep -E '^(test result:|---- |assertion|thread .* panicked)' /tmp/fc_test.log \
            | head -40 | sed 's/^/  /'
        FAILED+=("$combo:tests")
    fi
done

echo
echo "==================================================================="
if (( ${#FAILED[@]} == 0 )); then
    echo "ALL ${#COMBOS[@]} CONFIGURATION(S) PASSED"
    exit 0
else
    echo "FAILURES: ${FAILED[*]}"
    exit 1
fi
