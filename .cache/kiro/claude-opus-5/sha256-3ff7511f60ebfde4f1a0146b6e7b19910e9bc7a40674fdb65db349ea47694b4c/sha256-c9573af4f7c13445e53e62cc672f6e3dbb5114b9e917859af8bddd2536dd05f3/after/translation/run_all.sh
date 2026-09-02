#!/usr/bin/env bash
# Phase D driver: build the C .so, then run cargo check + the full differential
# suite for EVERY feature combination declared in Cargo.toml, in BOTH the
# release profile and the debug profile (debug enables Rust's integer-overflow
# checks and debug_assert!, so it independently proves the wrapping arithmetic
# never traps).
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
FAIL=0
ulimit -c 0 2>/dev/null || true   # UB probes fault on purpose; skip core dumps

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
echo "=== building C .so ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -1 "$ROOT"/c_src/build/*.so

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inx=1; next}
  /^\[/ {inx=0}
  inx && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}
' Cargo.toml)

if [[ -z "$FEATURES" ]]; then
    echo "=== Cargo.toml declares no [features]; the complete combination set is {default} ==="
    COMBOS=("default" "no-default")
else
    # Full power set of the declared features.
    mapfile -t FARR <<< "$FEATURES"
    N=${#FARR[@]}
    COMBOS=("default")
    for ((m=0; m<(1<<N); m++)); do
        combo=""
        for ((i=0; i<N; i++)); do
            if (( m & (1<<i) )); then combo+="${FARR[$i]},"; fi
        done
        COMBOS+=("no-default:${combo%,}")
    done
fi

printf '=== %d combination(s) to verify ===\n' "${#COMBOS[@]}"
printf '  - %s\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. For each combination x profile: check, build cdylib, symbol parity, test
# ---------------------------------------------------------------------------
run() {  # run <label> <profile-flag> <cargo feature args...>
    local label="$1"; shift
    local profile="$1"; shift
    local -a fargs=("$@")
    local pdir; [[ "$profile" == "--release" ]] && pdir=release || pdir=debug

    echo
    echo "################ $label [$pdir] ################"

    if ! timeout 600 cargo check $profile "${fargs[@]}" 2>&1 | tail -3; then
        echo "CHECK FAILED: $label [$pdir]"; FAIL=1; return
    fi
    if ! timeout 600 cargo build $profile "${fargs[@]}" 2>&1 | tail -3; then
        echo "BUILD FAILED: $label [$pdir]"; FAIL=1; return
    fi
    if ! ./check_symbols.sh "target/$pdir/libpow43_lib.so" | tail -3; then
        echo "SYMBOL PARITY FAILED: $label [$pdir]"; FAIL=1; return
    fi
    if ! timeout 600 cargo test $profile "${fargs[@]}" 2>&1 | grep -E 'test result|FAILED|panicked'; then
        echo "TEST FAILED: $label [$pdir]"; FAIL=1; return
    fi
    if timeout 600 cargo test $profile "${fargs[@]}" 2>&1 | grep -qE 'FAILED|test result: FAILED'; then
        echo "TEST FAILED: $label [$pdir]"; FAIL=1; return
    fi
}

for combo in "${COMBOS[@]}"; do
    case "$combo" in
        default)     args=() ;;
        no-default)  args=(--no-default-features) ;;
        no-default:) args=(--no-default-features) ;;
        no-default:*) args=(--no-default-features --features "${combo#no-default:}") ;;
        *)           args=(--features "$combo") ;;
    esac
    for profile in --release ""; do
        run "$combo" "$profile" "${args[@]}"
    done
done

echo
if (( FAIL )); then
    echo "########## RESULT: FAILURES PRESENT ##########"; exit 1
fi
echo "########## RESULT: ALL COMBINATIONS x PROFILES PASSED ##########"
