#!/usr/bin/env bash
# Full verification sweep: every build-time configuration of the Rust crate
# against every configuration of the C reference library.
#
#   ./verify.sh
#
# Cargo.toml declares no [features], so the feature powerset is the single
# empty combination -- that is enumerated (not assumed) below.
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
FAIL=0

note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
check() { if [ "$1" -eq 0 ]; then echo "   PASS  $2"; else echo "   FAIL  $2"; FAIL=1; fi; }

# --- 1. enumerate feature combinations ------------------------------------
FEATURES=$(python3 - <<'PY'
import re, itertools, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                names.append(n)
combos = [','.join(c) for n in range(len(names) + 1) for c in itertools.combinations(names, n)]
print('\n'.join(combos) if combos else '')
PY
)
note "feature combinations"
if [ -z "$FEATURES" ]; then
    echo "   (none declared -> single configuration: --no-default-features)"
    COMBOS=("")
else
    mapfile -t COMBOS <<<"$FEATURES"
    printf '   %s\n' "${COMBOS[@]}"
fi

# --- 2. cargo check for every combination ---------------------------------
note "cargo check"
for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
        timeout 600 cargo check --no-default-features >/tmp/check.log 2>&1
        check $? "cargo check --no-default-features"
    else
        timeout 600 cargo check --no-default-features --features "$combo" >/tmp/check.log 2>&1
        check $? "cargo check --features $combo"
    fi
done

# --- 3. build the C reference in several configurations --------------------
note "C reference builds"
declare -a C_SOS C_NAMES
timeout 600 cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 &&
    timeout 600 cmake --build "$ROOT/c_src/build" >>/tmp/cmake.log 2>&1
check $? "default (no optimisation)"
C_SOS+=("$(ls "$ROOT"/c_src/build/*.so | head -1)"); C_NAMES+=("default")
for opt in -O1 -O2 -O3 -Os; do
    d=/tmp/harvest_cbuild$opt
    timeout 600 cmake -S "$ROOT/c_src" -B "$d" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="$opt" >/tmp/cmake.log 2>&1 &&
        timeout 600 cmake --build "$d" >>/tmp/cmake.log 2>&1
    check $? "CFLAGS=$opt"
    C_SOS+=("$(ls "$d"/*.so | head -1)"); C_NAMES+=("$opt")
done

# --- 4/5. differential tests: every combo x every profile x every C build --
note "differential tests (Rust .so vs C .so via libloading)"
for combo in "${COMBOS[@]}"; do
    for profile in dev release; do
        relflag=(); [ "$profile" = release ] && relflag=(--release)
        featflag=(--no-default-features)
        [ -n "$combo" ] && featflag+=(--features "$combo")
        for i in "${!C_SOS[@]}"; do
            HARVEST_C_SO="${C_SOS[$i]}" timeout 600 cargo test -q \
                "${relflag[@]}" "${featflag[@]}" >/tmp/test.log 2>&1
            check $? "profile=$profile features='${combo:-<none>}' c=${C_NAMES[$i]}"
        done
    done
done

# --- 6. exported-symbol parity -------------------------------------------
note "exported symbol parity (nm -D)"
for profile in debug release; do
    RS=$(ls "target/$profile/libarr_ins_lib.so" "target/$profile/deps/libarr_ins_lib.so" 2>/dev/null | head -1)
    [ -n "$RS" ] || { echo "   SKIP  $profile (.so not built)"; continue; }
    nm -D --defined-only "$RS" | awk '{print $3}' | sort -u >/tmp/rs.txt
    for i in "${!C_SOS[@]}"; do
        nm -D --defined-only "${C_SOS[$i]}" | awk '{print $3}' | sort -u >/tmp/cs.txt
        missing=$(comm -23 /tmp/cs.txt /tmp/rs.txt)
        [ -z "$missing" ]
        check $? "$profile vs c=${C_NAMES[$i]}${missing:+ (missing: $(echo $missing))}"
    done
done

note "result"
if [ "$FAIL" -eq 0 ]; then echo "   all configurations match"; else echo "   FAILURES PRESENT"; fi
exit $FAIL
