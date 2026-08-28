#!/usr/bin/env bash
# Enumerates every valid [features] combination from Cargo.toml and runs
# `cargo check` + `cargo test` for each. With no [features] section the only
# valid configuration is the default one, which is still checked explicitly
# both with and without default features.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
LOG_DIR="${TMPDIR:-/tmp}/shputs-verify"
mkdir -p "$LOG_DIR"

# --- enumerate features -----------------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
text = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', text, re.M | re.S)
if not m:
    sys.exit(0)
for line in m.group(1).splitlines():
    line = line.split('#', 1)[0].strip()
    if not line or '=' not in line:
        continue
    name = line.split('=', 1)[0].strip().strip('"')
    if name and name != "default":
        print(name)
PY
)

N=${#FEATURES[@]}
echo "features found: $N ${FEATURES[*]:-(none)}"

COMBOS=()
if [ "$N" -eq 0 ]; then
    COMBOS+=("")
else
    for ((mask = 0; mask < (1 << N); mask++)); do
        combo=""
        for ((i = 0; i < N; i++)); do
            if (((mask >> i) & 1)); then
                combo="${combo:+$combo,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi

echo "combinations to verify: ${#COMBOS[@]}"

# --- build the C reference once ---------------------------------------------
(
    cd ../c_src || exit 1
    mkdir -p build && cd build || exit 1
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
) >"$LOG_DIR/c-build.log" 2>&1 || {
    echo "FAIL: C build (see $LOG_DIR/c-build.log)"
    exit 1
}

FAILED=0

run() { # run <label> <logfile> <cmd...>
    local label=$1 log=$2
    shift 2
    if timeout 600 "$@" >"$log" 2>&1; then
        echo "  ok   $label"
    else
        echo "  FAIL $label  ($log)"
        tail -n 25 "$log" | sed 's/^/       /'
        FAILED=1
    fi
}

for combo in "${COMBOS[@]}"; do
    label="${combo:-<no features>}"
    tag=$(echo "${combo:-none}" | tr ',' '_')
    echo "=== $label ==="

    run "check --no-default-features --features $label" \
        "$LOG_DIR/check-$tag.log" \
        cargo check --no-default-features ${combo:+--features "$combo"} --all-targets

    run "build cdylib" \
        "$LOG_DIR/build-$tag.log" \
        cargo build --release --no-default-features ${combo:+--features "$combo"}

    run "test" \
        "$LOG_DIR/test-$tag.log" \
        cargo test --no-default-features ${combo:+--features "$combo"}
done

# the default feature set, as an ordinary consumer would build it
echo "=== default features ==="
run "check (default)" "$LOG_DIR/check-default.log" cargo check --all-targets
run "build (default)" "$LOG_DIR/build-default.log" cargo build --release
run "test  (default)" "$LOG_DIR/test-default.log" cargo test

if [ "$FAILED" -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAILED"
