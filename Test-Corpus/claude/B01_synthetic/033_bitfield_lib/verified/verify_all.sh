#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination, check it compiles, then
# run the full differential suite (Phases B and C) against BOTH the debug and
# the release cdylib, and assert symbol parity with the C .so each time.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
FAIL=0
note() { printf '\n=== %s ===\n' "$*"; }

# --- 1. Enumerate feature combinations from Cargo.toml (the powerset) --------
mapfile -t FEATURES < <(python3 - "$ROOT/Cargo.toml" <<'PY' | grep -v '^$'
import sys, re
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name and name != 'default':
                feats.append(name)
print('\n'.join(feats))
PY
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
    # No [features] section => exactly one configuration.
    COMBOS=("")
    echo "Cargo.toml declares no [features]; exactly 1 configuration to verify."
else
    n=${#FEATURES[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        c=""
        for ((i = 0; i < n; i++)); do
            (((mask >> i) & 1)) && c+="${FEATURES[$i]},"
        done
        COMBOS+=("${c%,}")
    done
    echo "Found ${n} features -> ${#COMBOS[@]} combinations."
fi

# --- 2. Build the C reference library ---------------------------------------
note "Building the C shared library"
mkdir -p c_src/build
(cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$ROOT/target/c_syms.txt"
echo "C .so exports: $(tr '\n' ' ' < "$ROOT/target/c_syms.txt")"

# --- 3. Per combination: check, symbol parity, and the differential suite ----
for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
        FARGS=(--no-default-features)
        label="<no features>"
    else
        FARGS=(--no-default-features --features "$combo")
        label="$combo"
    fi

    note "cargo check --no-default-features ${combo:+--features $combo}"
    if ! timeout 600 cargo check "${FARGS[@]}" 2>&1 | tail -3; then
        echo "CHECK FAILED for [$label]"; FAIL=1; continue
    fi

    for profile in debug release; do
        note "[$label] profile=$profile"
        PARGS=("${FARGS[@]}")
        [ "$profile" = release ] && PARGS+=(--release)

        if ! timeout 600 cargo build "${PARGS[@]}" >/dev/null 2>&1; then
            echo "BUILD FAILED for [$label/$profile]"; FAIL=1; continue
        fi
        RS_SO="$ROOT/target/$profile/libdriver.so"

        # Symbol parity: every C symbol must be exported by the Rust .so.
        nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort -u > "$ROOT/target/rs_syms.txt"
        missing=$(comm -23 "$ROOT/target/c_syms.txt" "$ROOT/target/rs_syms.txt")
        if [ -n "$missing" ]; then
            echo "SYMBOL PARITY FAILED [$label/$profile]; missing: $missing"; FAIL=1
        else
            echo "symbol parity OK (0 missing)"
        fi
        # No undefined non-libc symbols. Strip the @GLIBC_x.y version suffix
        # first, then drop everything that is a libc/compiler-runtime import.
        undef=$(nm -D --undefined-only "$RS_SO" | awk '{print $NF}' \
            | sed 's/@.*$//' \
            | grep -v '^$' \
            | grep -vE '^(__|_Z|_ITM_|_GLOBAL__)' \
            | grep -vE '^(printf|memcpy|memmove|memset|memcmp|bcmp|strlen|malloc|calloc|realloc|free|posix_memalign|aligned_alloc|abort|exit|_exit)$' \
            | grep -vE '^(write|writev|read|close|open|open64|fstat|fstat64|stat|stat64|statx|lseek|lseek64|mmap|mmap64|munmap|mprotect|readlink|realpath|getcwd|getenv|poll|syscall|sysconf)$' \
            | grep -vE '^(sigaction|sigaltstack|sigemptyset|sigaddset|raise|dl_iterate_phdr|dlsym|gettid|getpid|sched_yield|nanosleep|clock_gettime)$' \
            | grep -vE '^pthread_[a-z_]+$' \
            | grep -vE '^_Unwind_[A-Za-z]+$')
        # Definitive check: the dynamic linker resolves everything.
        if ldd -r "$RS_SO" 2>&1 | grep -qi 'undefined\|not found'; then
            echo "UNRESOLVED SYMBOLS at load time [$label/$profile]"; FAIL=1
        else
            echo "ldd -r OK (all dynamic symbols resolve)"
        fi
        if [ -n "$undef" ]; then
            echo "NOTE: undefined symbols not on the libc allowlist:"; echo "$undef"
        else
            echo "undefined-symbol check OK (libc only)"
        fi

        # Phases B and C against exactly this .so.
        if DRIVER_RUST_SO="$RS_SO" timeout 600 cargo test "${FARGS[@]}" 2>&1 \
                | grep -E "^(test result|running|test .* FAILED)"; then
            :
        fi
        if ! DRIVER_RUST_SO="$RS_SO" timeout 600 cargo test "${FARGS[@]}" >/dev/null 2>&1; then
            echo "DIFFERENTIAL TESTS FAILED [$label/$profile]"; FAIL=1
        else
            echo "differential tests PASSED [$label/$profile]"
        fi
    done
done

note "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED (${#COMBOS[@]} feature combination(s) x 2 profiles)"
else
    echo "FAILURES PRESENT"
fi
exit "$FAIL"
