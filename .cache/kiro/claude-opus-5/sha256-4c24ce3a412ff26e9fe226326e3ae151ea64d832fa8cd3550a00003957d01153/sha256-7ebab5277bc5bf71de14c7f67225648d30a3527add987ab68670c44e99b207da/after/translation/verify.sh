#!/usr/bin/env bash
# Differential verification driver.
#
#   1. enumerates every valid feature combination declared in Cargo.toml
#   2. `cargo check`s each one
#   3. builds the C reference library with cmake
#   4. runs the differential test suite for each combination, against both the
#      debug and the release Rust cdylib
#   5. diffs `nm -D` between the two shared objects
#
# Usage: ./verify.sh [--quick]
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
TIMEOUT=${TIMEOUT:-600}
FAILURES=0

note() { printf '\n== %s\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILURES=$((FAILURES + 1)); }

# --------------------------------------------------------------------------
# 1. Enumerate feature combinations (the powerset of the declared features).
# --------------------------------------------------------------------------
mapfile -t RAW_FEATURES < <(python3 - <<'PY'
import re
src = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        key = line.split('=', 1)[0].strip().strip('"')
        if key and key != "default":
            names.append(key)
print("\n".join(names))
PY
)

# `mapfile` yields a single empty element for empty input - drop blanks.
FEATURES=()
for f in "${RAW_FEATURES[@]}"; do
    [ -n "$f" ] && FEATURES+=("$f")
done

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
    # No [features] table: the crate has exactly one build configuration.
    COMBOS=("")
else
    n=${#FEATURES[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (((mask >> i) & 1)); then
                combo="${combo:+$combo,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi

note "declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"
note "feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '   - [%s]\n' "${c:-<none>}"; done

# --------------------------------------------------------------------------
# 2. cargo check every combination.
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    note "cargo check --no-default-features --features '${combo:-<none>}'"
    if ! timeout "$TIMEOUT" cargo check --no-default-features \
        ${combo:+--features "$combo"} --all-targets >/tmp/check.log 2>&1; then
        tail -30 /tmp/check.log
        fail "cargo check [${combo:-<none>}]"
    fi
done

# --------------------------------------------------------------------------
# 3. Build the C reference library.
# --------------------------------------------------------------------------
note "building the C reference library"
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" &&
    timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 &&
    timeout "$TIMEOUT" cmake --build . >>/tmp/cmake.log 2>&1) ||
    { tail -20 /tmp/cmake.log; fail "cmake build"; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)
printf '   C .so: %s\n' "$C_SO"

# --------------------------------------------------------------------------
# 4. Run the suite for every combination, against both cdylib profiles.
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    for profile in debug release; do
        note "cargo test [${combo:-<none>}] against the $profile cdylib"

        build_args=(--no-default-features)
        [ -n "$combo" ] && build_args+=(--features "$combo")
        [ "$profile" = release ] && build_args+=(--release)

        if ! timeout "$TIMEOUT" cargo build --lib "${build_args[@]}" \
            >/tmp/build-"$profile".log 2>&1; then
            tail -30 /tmp/build-"$profile".log
            fail "cargo build --lib [$combo/$profile]"
            continue
        fi
        so="$PWD/target/$profile/libreverse_collide_lib.so"
        [ -f "$so" ] || { fail "missing $so"; continue; }

        test_args=(--no-default-features)
        [ -n "$combo" ] && test_args+=(--features "$combo")
        if ! RUST_SO_PATH="$so" C_SO_PATH="$C_SO" \
            timeout "$TIMEOUT" cargo test "${test_args[@]}" --no-fail-fast \
            >/tmp/test.log 2>&1; then
            grep -E 'FAILED|mismatch|panicked' /tmp/test.log | sort -u | head -20
            fail "cargo test [${combo:-<none>}/$profile]"
        else
            grep -E '^test result' /tmp/test.log | sed 's/^/   /'
        fi

        # ------------------------------------------------------------------
        # 5. Symbol parity for this exact pair of shared objects.
        # ------------------------------------------------------------------
        nm -D --defined-only --format=posix "$C_SO" | awk '$2 ~ /^[TDBRWi]$/ {print $1}' |
            sort -u >/tmp/c_syms.txt
        nm -D --defined-only --format=posix "$so" | awk '$2 ~ /^[TDBRWi]$/ {print $1}' |
            sort -u >/tmp/rs_syms.txt
        missing=$(comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt |
            grep -vE '^(_init|_fini|__bss_start|_edata|_end)$' || true)
        if [ -n "$missing" ]; then
            printf '   symbols exported by C but not by Rust:\n%s\n' "$missing"
            fail "symbol parity [${combo:-<none>}/$profile]"
        else
            printf '   symbol parity OK (%s C symbols all present)\n' "$(wc -l </tmp/c_syms.txt)"
        fi
    done
done

note "done"
if [ "$FAILURES" -eq 0 ]; then
    echo "ALL CHECKS PASSED"
    exit 0
fi
echo "$FAILURES CHECK(S) FAILED"
exit 1
