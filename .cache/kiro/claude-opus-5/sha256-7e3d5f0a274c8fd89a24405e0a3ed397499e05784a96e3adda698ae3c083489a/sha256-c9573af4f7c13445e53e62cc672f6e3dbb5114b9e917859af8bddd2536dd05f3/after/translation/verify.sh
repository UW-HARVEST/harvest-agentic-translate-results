#!/usr/bin/env bash
# Phase D driver: symbol parity + full differential suite under EVERY cargo
# feature combination. Enumerates features from Cargo.toml rather than
# hard-coding them, so a newly added feature is picked up automatically.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libsodium.so"
R_SO="target/release/liblibsodium.so"
fail=0

echo "=== 0. Build the C reference .so ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . -j 8 >/dev/null 2>&1 ) || { echo "C build FAILED"; exit 1; }
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }
echo "ok: $C_SO"

# --- enumerate features -----------------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name and name != 'default':
                print(name)
PY
)

# Build the list of combinations to test: the default build, plus (if any
# features exist) the powerset of them with --no-default-features.
COMBOS=("__default__")
if [ "${#FEATURES[@]}" -gt 0 ]; then
    n=${#FEATURES[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (((mask >> i) & 1)); then combo="$combo,${FEATURES[$i]}"; fi
        done
        COMBOS+=("${combo#,}")
    done
fi

echo
echo "=== features declared: ${#FEATURES[@]} ${FEATURES[*]:-(none)} ==="
echo "=== configurations to verify: ${#COMBOS[@]} ==="

for combo in "${COMBOS[@]}"; do
    if [ "$combo" = "__default__" ]; then
        LABEL="default"
        ARGS=()
    else
        LABEL="--no-default-features --features '${combo:-<none>}'"
        ARGS=(--no-default-features)
        [ -n "$combo" ] && ARGS+=(--features "$combo")
    fi

    echo
    echo "############################################################"
    echo "# CONFIG: $LABEL"
    echo "############################################################"

    echo "--- cargo check"
    if ! cargo check --release "${ARGS[@]}" 2>&1 | tail -3; then fail=1; fi

    echo "--- cargo build"
    if ! cargo build --release "${ARGS[@]}" 2>&1 | tail -3; then
        echo "BUILD FAILED for $LABEL"; fail=1; continue
    fi

    echo "--- symbol parity (nm -D)"
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/pd_c.txt
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/pd_r.txt
    missing=$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)
    extra=$(comm -13 /tmp/pd_c.txt /tmp/pd_r.txt)
    printf "    C=%s Rust=%s missing=%s extra=%s\n" \
        "$(wc -l < /tmp/pd_c.txt)" "$(wc -l < /tmp/pd_r.txt)" \
        "$(printf '%s' "$missing" | grep -c . )" "$(printf '%s' "$extra" | grep -c .)"
    if [ -n "$missing" ]; then echo "MISSING FROM RUST:"; echo "$missing"; fail=1; fi

    echo "--- undefined non-libc symbols in Rust .so"
    und=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' \
        | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^_Unwind_' || true)
    if [ -n "$und" ]; then echo "UNRESOLVED: $und"; fail=1; else echo "    none"; fi

    echo "--- differential test suite"
    if ! timeout 600 cargo test --release "${ARGS[@]}" 2>&1 \
            | grep -E '^(test result|error)|FAILED'; then fail=1; fi
    if timeout 600 cargo test --release "${ARGS[@]}" 2>&1 | grep -q 'FAILED'; then fail=1; fi
done

echo
if [ "$fail" -eq 0 ]; then
    echo "=== ALL CONFIGURATIONS PASS ==="
else
    echo "=== FAILURES PRESENT ==="
fi
exit "$fail"
