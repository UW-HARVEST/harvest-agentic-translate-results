#!/usr/bin/env bash
# Full verification driver: enumerates every build configuration and runs the
# differential suite for each one.
#
#   ./verify_all.sh
#
# Steps
#   1. enumerate every valid cargo feature combination (powerset of [features])
#   2. `cargo check` each combination
#   3. build the C shared library (default cmake config, and a -O2 one
#      out-of-tree so nothing in c_src/ is touched)
#   4. build the Rust cdylib for the dev and release profiles
#   5. run the differential suite for the whole {rust profile} x {c build}
#      x {feature combination} matrix
set -uo pipefail
cd "$(dirname "$0")"
: "${TMPDIR:=/tmp}"
LOG=${TMPDIR%/}/verify_all.log
: > "$LOG"
fail=0

say() { printf '\n=== %s ===\n' "$*"; }
run() { # run <description> <cmd...>
    local desc=$1; shift
    printf '%-72s' "$desc"
    if timeout 600 "$@" >>"$LOG" 2>&1; then
        echo "OK"
    else
        echo "FAIL  (see $LOG)"
        fail=1
    fi
}

# ---------------------------------------------------------------- 1. features
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        name = line.split('=')[0].strip()
        if name and name != 'default':
            feats.append(name)
combos = []
for n in range(len(feats) + 1):
    for c in itertools.combinations(feats, n):
        combos.append(','.join(c))
if not combos:
    combos = ['']
print('\n'.join(combos))
PY
)
say "feature combinations (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do echo "  --no-default-features --features '${c}'"; done

# ------------------------------------------------------------------ 2. check
say "cargo check for every combination"
for c in "${COMBOS[@]}"; do
    run "cargo check --no-default-features --features '${c}'" \
        cargo check --no-default-features --features "$c"
    run "cargo check --tests --no-default-features --features '${c}'" \
        cargo check --tests --no-default-features --features "$c"
done
run "cargo check (default features)" cargo check
run "cargo check --all-features" cargo check --all-features

# ------------------------------------------------------------------ 3. C libs
say "build the C shared library"
mkdir -p c_src/build
run "cmake configure (default)" cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
run "cmake build (default)"     cmake --build c_src/build
C_DEFAULT=$PWD/c_src/build/libtranslated_rust.so

C_O2_DIR=${TMPDIR%/}/c_build_release
run "cmake configure (-O2, out of tree)" \
    cmake -S c_src -B "$C_O2_DIR" -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_BUILD_TYPE=Release
run "cmake build (-O2, out of tree)" cmake --build "$C_O2_DIR"
C_O2=$C_O2_DIR/libtranslated_rust.so

# --------------------------------------------------------------- 4./5. matrix
say "differential suite over the full matrix"
for c in "${COMBOS[@]}"; do
    for prof in dev release; do
        if [ "$prof" = dev ]; then
            run "cargo build --no-default-features --features '${c}' (dev)" \
                cargo build --no-default-features --features "$c"
            RUST_SO=$PWD/target/debug/libfindrep_lib.so
        else
            run "cargo build --release --no-default-features --features '${c}'" \
                cargo build --release --no-default-features --features "$c"
            RUST_SO=$PWD/target/release/libfindrep_lib.so
        fi
        for cname in default O2; do
            [ "$cname" = default ] && C_SO=$C_DEFAULT || C_SO=$C_O2
            run "test  features='${c}' rust=${prof} c=${cname}" \
                env "HARVEST_RUST_LIB=$RUST_SO" "HARVEST_C_LIB=$C_SO" \
                cargo test --no-default-features --features "$c" -- --quiet
            # opt-in: the exhaustive/wide sweeps (all 2^32 inputs for
            # validate_and_normalize, millions for the other leaves)
            if [ "${RUN_EXHAUSTIVE:-0}" = 1 ]; then
                relflag=""
                [ "$prof" = release ] && relflag="--release"
                run "sweep features='${c}' rust=${prof} c=${cname}" \
                    env "HARVEST_RUST_LIB=$RUST_SO" "HARVEST_C_LIB=$C_SO" \
                    cargo test $relflag --no-default-features --features "$c" -- \
                    --ignored exhaustive_ --quiet
            fi
        done
    done
done

say "summary"
if [ "$fail" = 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "FAILURES PRESENT — grep for 'FAILED\\|error' in $LOG"
fi
exit "$fail"
