#!/usr/bin/env bash
# Phase D — enumerate every Cargo feature combination mechanically and run the
# full differential suite under each. Also runs the suite against BOTH builds of
# the Rust cdylib (dev = debug assertions + overflow checks, release =
# panic=abort + optimised), since that is the only other build axis this crate
# has.
set -uo pipefail
cd "$(dirname "$0")"

C_SO="$(cd .. && pwd)/c_src/build/libdriver.so"
if [[ ! -f "$C_SO" ]]; then
    echo "building the C shared library..."
    (cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null)
fi
export C_DRIVER_SO="$C_SO"

# --- enumerate features from Cargo.toml -------------------------------------
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { inblock = 1; next }
        /^\[/           { inblock = 0 }
        inblock && /=/  { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                          if (a[1] != "default" && a[1] != "") print a[1] }
    ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# every subset of FEATURES (2^n); n is small by construction
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
    [[ -z "$f" ]] && continue
    new=()
    for c in "${COMBOS[@]}"; do
        new+=("$c")
        if [[ -z "$c" ]]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    COMBOS=("${new[@]}")
done

fail=0

run_suite () {   # $1 = human label, $2... = extra cargo args
    local label="$1"; shift
    for profile in dev release; do
        local prof_args=() so
        if [[ "$profile" == release ]]; then
            prof_args=(--release); so="target/release/libdriver.so"
        else
            so="target/debug/libdriver.so"
        fi
        cargo build "${prof_args[@]}" "$@" -q || { echo "BUILD FAIL $label/$profile"; fail=1; continue; }
        RUST_DRIVER_SO="$(pwd)/$so" \
            timeout 600 cargo test "${prof_args[@]}" "$@" -q >/tmp/ft.$$.log 2>&1
        local rc=$?
        local summary
        summary=$(grep -c '^test result: ok' /tmp/ft.$$.log)
        if [[ $rc -eq 0 ]]; then
            echo "PASS  [$label] profile=$profile ($summary test binaries ok)"
        else
            echo "FAIL  [$label] profile=$profile (rc=$rc)"
            tail -30 /tmp/ft.$$.log
            fail=1
        fi
        rm -f /tmp/ft.$$.log
    done
}

for combo in "${COMBOS[@]}"; do
    if [[ -z "$combo" ]]; then
        run_suite "default"                 # default features
        run_suite "no-default-features" --no-default-features
    else
        run_suite "features=$combo" --no-default-features --features "$combo"
    fi
done

if [[ ${#FEATURES[@]} -gt 0 ]]; then
    run_suite "all-features" --all-features
fi

echo
if [[ $fail -eq 0 ]]; then
    echo "ALL FEATURE COMBINATIONS PASSED"
else
    echo "SOME COMBINATIONS FAILED"
fi
exit $fail
