#!/usr/bin/env bash
# Phase D driver: run the full differential suite under EVERY feature
# combination declared in Cargo.toml, against BOTH Rust build profiles, and
# check symbol parity each time.
#
# Usage: ./check_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
C_SO="$ROOT/../c_src/build/libdriver.so"
FAIL=0

say() { printf '\n=== %s ===\n' "$*"; }

# --- Enumerate feature combinations from Cargo.toml -------------------------
# Every explicit feature, plus the empty set. `driver` declares no [features]
# section, so this yields the single default configuration; the loop is written
# generically so it keeps working if features are added later.
FEATURES=$(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name and name != 'default':
                feats.append(name)
print(' '.join(feats))
PY
)
say "declared features: [${FEATURES:-none}]"

COMBOS=()
COMBOS+=("--no-default-features")      # empty feature set
COMBOS+=("")                            # default feature set
if [ -n "$FEATURES" ]; then
    # every single feature, and the all-features build
    for f in $FEATURES; do
        COMBOS+=("--no-default-features --features $f")
    done
    COMBOS+=("--all-features")
fi

# --- Build the C reference once ---------------------------------------------
if [ ! -f "$C_SO" ]; then
    say "building C reference"
    (cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi

C_SYMS=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u)

for combo in "${COMBOS[@]}"; do
    label="${combo:-<default features>}"

    say "cargo check [$label]"
    if ! timeout 600 cargo check $combo >/dev/null 2>&1; then
        echo "FAIL: cargo check [$label]"; FAIL=1; continue
    fi

    for profile in debug release; do
        say "profile=$profile features=[$label]"
        relflag=""
        [ "$profile" = release ] && relflag="--release"
        if ! timeout 600 cargo build $relflag $combo >/dev/null 2>&1; then
            echo "FAIL: cargo build $profile [$label]"; FAIL=1; continue
        fi

        RUST_SO="$ROOT/target/$profile/libdriver.so"
        if [ ! -f "$RUST_SO" ]; then
            echo "FAIL: missing $RUST_SO"; FAIL=1; continue
        fi

        # --- symbol parity -------------------------------------------------
        R_SYMS=$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u)
        MISSING=$(comm -23 <(printf '%s\n' "$C_SYMS") <(printf '%s\n' "$R_SYMS"))
        if [ -n "$MISSING" ]; then
            echo "FAIL: symbols exported by C but not Rust ($profile/$label):"
            printf '  %s\n' $MISSING
            FAIL=1
        else
            echo "symbol parity OK ($(printf '%s\n' "$C_SYMS" | wc -l) C symbols all present)"
        fi
        # undefined non-libc symbols in the Rust .so
        UNDEF=$(nm -D -u "$RUST_SO" | awk '{print $2}' \
            | grep -vE '^(_Unwind_|_ITM_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location)' \
            | grep -vE '^(memcpy|memmove|memset|bcmp|strlen|printf|malloc|calloc|realloc|free|posix_memalign|abort|getenv|getcwd|readlink|realpath|open64|close|read|write|writev|lseek64|fstat64|stat64|statx|mmap64|munmap|syscall|dl_iterate_phdr|gettid|pthread_[a-z_]+)(@.*)?$' \
            | sed 's/@.*//' | sort -u)
        if [ -n "$UNDEF" ]; then
            echo "FAIL: undefined non-libc symbols in Rust .so:"; printf '  %s\n' $UNDEF; FAIL=1
        fi

        # --- differential tests against THIS .so ---------------------------
        # Tests always build in the dev profile (the release profile sets
        # panic=abort, which the test harness cannot use); DRIVER_RUST_SO pins
        # which shared object they load.
        for suite in differential errors; do
            log="$ROOT/target/test-$suite-$profile.log"
            DRIVER_RUST_SO="$RUST_SO" timeout 600 cargo test $combo \
                --test "$suite" -- --test-threads=4 >"$log" 2>&1
            rc=$?
            grep -E '^test result:' "$log" | sed "s/^/  $suite: /"
            if [ $rc -ne 0 ]; then
                echo "FAIL: $suite tests ($profile/$label), see $log"
                grep -E '^(failures:|----|thread .* panicked)' "$log" | head -20
                FAIL=1
            fi
        done
    done
done

say "RESULT"
if [ "$FAIL" -eq 0 ]; then
    echo "ALL CHECKS PASSED"
else
    echo "FAILURES PRESENT"
fi
exit "$FAIL"
