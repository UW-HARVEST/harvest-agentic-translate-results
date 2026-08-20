#!/usr/bin/env bash
# Full verification driver: enumerates every cargo feature combination declared in
# Cargo.toml, then for each one runs cargo check, builds the cdylib, diffs `nm -D`
# against the C .so, and runs the differential suite.
set -u
cd "$(dirname "$0")"
export CARGO_NET_OFFLINE=true
FAIL=0

# ---- enumerate feature combinations -------------------------------------------
# Read the [features] table out of Cargo.toml (excluding "default") and build the
# power set. An empty table yields exactly one combination: the empty set.
mapfile -t FEATS < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
print('\n'.join(names))
PY
)
# drop empty lines
COMBOS=()
if [ "${#FEATS[@]}" -eq 0 ] || [ -z "${FEATS[0]:-}" ]; then
    COMBOS=("")
    echo "Cargo.toml declares no [features] -> exactly 1 feature combination (empty set)"
else
    n=${#FEATS[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        c=""
        for ((i = 0; i < n; i++)); do
            if (((mask >> i) & 1)); then c="${c:+$c,}${FEATS[$i]}"; fi
        done
        COMBOS+=("$c")
    done
    echo "features: ${FEATS[*]}  ->  ${#COMBOS[@]} combinations"
fi

# ---- build the C reference library --------------------------------------------
echo "=============================================================="
echo "Building the C reference shared library"
echo "=============================================================="
(mkdir -p c_src/build && cd c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null) || { echo "C BUILD FAILED"; exit 1; }
C_SO=c_src/build/libdriver.so
ls -l "$C_SO"

run_combo() {
    local combo="$1" profile="$2"
    local fflag="--no-default-features"
    [ -n "$combo" ] && fflag="--no-default-features --features $combo"
    local pflag=""
    local dir="debug"
    if [ "$profile" = "release" ]; then pflag="--release"; dir="release"; fi

    echo
    echo "=============================================================="
    echo "combo: [${combo:-<none>}]   profile: $profile"
    echo "=============================================================="

    echo "--- cargo check ---"
    if ! timeout 600 cargo check $fflag $pflag --tests 2>&1 | tail -5; then
        echo "CHECK FAILED"; FAIL=1; return
    fi

    echo "--- cargo build (cdylib) ---"
    if ! timeout 600 cargo build $fflag $pflag 2>&1 | tail -3; then
        echo "BUILD FAILED"; FAIL=1; return
    fi

    echo "--- nm -D symbol diff (C vs Rust) ---"
    local R_SO="target/$dir/libdriver.so"
    local d
    d=$(diff <(nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort) \
             <(nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort))
    if [ -n "$d" ]; then
        echo "SYMBOL DIFF NOT EMPTY:"; echo "$d"; FAIL=1
    else
        echo "symbol diff EMPTY ($(nm -D --defined-only "$C_SO" | awk '$2=="T"' | wc -l) symbols match)"
    fi

    echo "--- cargo test (differential suite) ---"
    if timeout 600 cargo test $fflag $pflag 2>&1 | tail -14; then
        :
    else
        echo "TESTS FAILED"; FAIL=1
    fi
}

for combo in "${COMBOS[@]}"; do
    for profile in debug release; do
        run_combo "$combo" "$profile"
    done
done

echo
if [ "$FAIL" -eq 0 ]; then
    echo "###### ALL CONFIGURATIONS VERIFIED ######"
else
    echo "###### VERIFICATION FAILED ######"
fi
exit "$FAIL"
