#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff exported symbols, and run the
# full Phase B + Phase C differential suites under every feature combination
# and both profiles. Nothing here is manual/per-configuration -- the feature
# combinations are extracted from Cargo.toml.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT=$(cd .. && pwd)
FAIL=0

echo "=============================================================="
echo "1. Rebuild the C shared library"
echo "=============================================================="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
CSO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)
echo "C  .so: $CSO"

echo
echo "=============================================================="
echo "2. Enumerate feature combinations from Cargo.toml"
echo "=============================================================="
# Every feature declared under [features], excluding the `default` meta-feature.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
    echo "No [features] declared -> the only configurations that exist are the"
    echo "default one and --no-default-features (which are identical here)."
    COMBOS=("" "--no-default-features")
else
    COMBOS=("" "--no-default-features")
    for f in $FEATURES; do
        COMBOS+=("--no-default-features --features $f")
    done
    # all features at once
    COMBOS+=("--all-features")
fi
printf 'combination: %s\n' "${COMBOS[@]/#/[default] }"

for PROFILE in debug release; do
  PROFILE_FLAG=""
  [ "$PROFILE" = release ] && PROFILE_FLAG="--release"
  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features='${COMBO:-<default>}'"
    echo
    echo "=============================================================="
    echo "3. $LABEL"
    echo "=============================================================="

    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $PROFILE_FLAG $COMBO >/dev/null 2>&1; then
        echo "  cargo build FAILED for $LABEL"; FAIL=1; continue
    fi
    RSO="target/$PROFILE/libcontrast_ratio_lib.so"
    if [ ! -f "$RSO" ]; then echo "  missing $RSO"; FAIL=1; continue; fi

    # --- symbol parity ---
    nm -D --defined-only "$CSO"  | awk '$2 ~ /^[TtWwDdBbRr]$/ {print $3}' | sort -u > /tmp/c_syms.txt
    nm -D --defined-only "$RSO"  | awk '$2 ~ /^[TtWwDdBbRr]$/ {print $3}' | sort -u > /tmp/r_syms.txt
    MISSING=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
    if [ -n "$MISSING" ]; then
        echo "  SYMBOL PARITY FAILED -- missing from Rust .so:"; echo "$MISSING" | sed 's/^/    /'
        FAIL=1
    else
        echo "  symbol parity OK ($(wc -l < /tmp/c_syms.txt) C symbol(s), 0 missing)"
    fi

    # undefined non-libc symbols in the Rust .so
    UNDEF=$(nm -D -u "$RSO" | awk '{print $2}' | sed 's/@.*//' \
        | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location|_+[A-Za-z]*_?$)' \
        | grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat|fstat64|getcwd|getenv|gettid|lseek|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap|mmap64|munmap|open|open64|posix_memalign|pow|pthread_[a-z_]*|read|readlink|realloc|realpath|stat|stat64|statx|strlen|syscall|write|writev|sysconf|qsort|snprintf|__[a-z_0-9]*' \
        | sort -u)
    if [ -n "$UNDEF" ]; then
        echo "  NOTE: undefined symbols not recognised as libc/unwinder:"; echo "$UNDEF" | sed 's/^/    /'
    else
        echo "  undefined non-libc symbols: 0"
    fi

    # --- differential suites ---
    for T in phase_a_harness phase_b_configs phase_c_errors; do
        # shellcheck disable=SC2086
        OUT=$(timeout 600 cargo test $PROFILE_FLAG $COMBO --test "$T" 2>&1)
        LINE=$(echo "$OUT" | grep -E '^test result:' | tail -1)
        if echo "$OUT" | grep -qE '^test result: ok'; then
            echo "  $T: $LINE"
        else
            echo "  $T: FAILED -- $LINE"
            echo "$OUT" | grep -A3 -E '^failures:|panicked at' | head -40 | sed 's/^/    /'
            FAIL=1
        fi
    done
  done
done

echo
echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
    echo "PHASE D: ALL CONFIGURATIONS PASS"
else
    echo "PHASE D: FAILURES PRESENT (see above)"
fi
echo "=============================================================="
exit "$FAIL"
