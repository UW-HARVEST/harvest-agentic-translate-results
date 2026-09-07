#!/usr/bin/env bash
# Phase D driver: rebuild both shared objects, diff their exported symbols, and
# run the whole differential suite under EVERY feature combination declared in
# Cargo.toml. Feature combinations are extracted from Cargo.toml, never
# hard-coded.
set -uo pipefail

cd "$(dirname "$0")/.."          # translation/
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=== building the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C  .so: $C_SO"

echo
echo "=== enumerating feature combinations from Cargo.toml ==="
mapfile -t FEATURES < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', txt, re.S | re.M)
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
if [ "${#FEATURES[@]}" -eq 0 ]; then
    echo "no [features] table -> the only configuration is the default one"
    COMBOS=("default")
else
    echo "features: ${FEATURES[*]}"
    COMBOS=()
    n=${#FEATURES[@]}
    for ((mask=0; mask<(1<<n); mask++)); do
        combo=""
        for ((i=0; i<n; i++)); do
            if (( mask & (1<<i) )); then combo="$combo,${FEATURES[$i]}"; fi
        done
        COMBOS+=("${combo#,}")
    done
    COMBOS+=("default")
fi

for combo in "${COMBOS[@]}"; do
    echo
    echo "############################################################"
    if [ "$combo" = "default" ]; then
        FLAGS=()
        echo "# combination: <default features>"
    elif [ -z "$combo" ]; then
        FLAGS=(--no-default-features)
        echo "# combination: --no-default-features"
    else
        FLAGS=(--no-default-features --features "$combo")
        echo "# combination: --no-default-features --features $combo"
    fi
    echo "############################################################"

    timeout 600 cargo check --release "${FLAGS[@]}" >/dev/null 2>&1 \
        || { echo "cargo check FAILED"; FAIL=1; continue; }
    timeout 600 cargo build --release "${FLAGS[@]}" >/dev/null 2>&1 \
        || { echo "cargo build FAILED"; FAIL=1; continue; }

    R_SO=target/release/libhm_geti_lib.so
    echo "--- symbol diff (C .so vs Rust .so) ---"
    diff <(nm -D --defined-only "$C_SO" | awk '{print $2, $3}' | sort) \
         <(nm -D --defined-only "$R_SO" | awk '{print $2, $3}' | sort) \
        && echo "symbol diff: EMPTY (parity)" \
        || { echo "SYMBOL DIFF NOT EMPTY"; FAIL=1; }

    echo "--- undefined non-libc symbols in the Rust .so ---"
    UNDEF="$(nm -D --undefined-only "$R_SO" | awk '{print $2}' \
        | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location)' \
        | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_[a-z_]+|read|readlink|realloc|realpath|stat64|statx|strcmp|strlen|syscall|write|writev)(@|$)' \
        || true)"
    if [ -z "$UNDEF" ]; then echo "none"; else echo "$UNDEF"; FAIL=1; fi

    for t in harness_selftest phase_b_hash phase_b_arr phase_b_map phase_b_strings phase_b_driver phase_c_errors; do
        echo "--- cargo test --test $t ---"
        timeout 600 cargo test --release "${FLAGS[@]}" --test "$t" -- --test-threads=1 2>&1 \
            | grep -E '^(test |test result|error)' | grep -vE '^test .* \.\.\. ok$'
        # shellcheck disable=SC2181
        if [ "${PIPESTATUS[0]}" -ne 0 ]; then echo "TEST BINARY $t FAILED"; FAIL=1; fi
    done
done

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
