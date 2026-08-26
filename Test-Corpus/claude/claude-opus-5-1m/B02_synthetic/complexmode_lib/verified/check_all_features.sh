#!/usr/bin/env bash
# Phase A / Phase D driver: enumerate every valid Cargo feature combination and
# run `cargo check` + the full differential test suite for each one.
#
# `Cargo.toml` has no [features] table, so the power set is {(no features)},
# which is also the default build.  The script derives that mechanically
# instead of hard-coding it, so it keeps working if features are ever added.
set -u -o pipefail

cd "$(dirname "$0")" || exit 1
LOG_DIR="${TMPDIR:-/tmp}/feature-matrix"
mkdir -p "$LOG_DIR"

# ---------------------------------------------------------------------------
# 1. build the C reference shared object
# ---------------------------------------------------------------------------
if [ ! -f c_src/build/libtranslated_rust.so ]; then
    ( mkdir -p c_src/build && cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && cmake --build . ) > "$LOG_DIR/cmake.log" 2>&1 \
      || { echo "FAIL: C build (see $LOG_DIR/cmake.log)"; exit 1; }
fi
echo "C  .so: c_src/build/libtranslated_rust.so"

# ---------------------------------------------------------------------------
# 2. enumerate features declared in Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "=");
        gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "declared features: $n ${FEATURES[*]:-(none)}"

# power set of FEATURES (2^n combinations, n == 0 -> just the empty combo)
COMBOS=()
for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
        if (( (mask >> i) & 1 )); then
            combo="${combo:+$combo,}${FEATURES[$i]}"
        fi
    done
    COMBOS+=("$combo")
done

status=0
run() { # run <label> <logfile> <cmd...>
    local label="$1" log="$2"; shift 2
    if timeout 600 "$@" > "$log" 2>&1; then
        echo "  PASS  $label"
    else
        echo "  FAIL  $label   (log: $log)"
        tail -n 25 "$log" | sed 's/^/        /'
        status=1
    fi
}

for combo in "${COMBOS[@]}"; do
    label="--no-default-features --features '${combo}'"
    echo "=== $label ==="
    safe=$(echo "${combo:-none}" | tr ',' '_')
    run "cargo check  [$safe]" "$LOG_DIR/check-$safe.log" \
        cargo check --no-default-features --features "$combo" --all-targets
    run "cargo build  [$safe]" "$LOG_DIR/build-$safe.log" \
        cargo build --no-default-features --features "$combo"
    run "cargo test   [$safe]" "$LOG_DIR/test-$safe.log" \
        cargo test --no-default-features --features "$combo" -- --test-threads=1
done

# the plain default configuration too (identical here, but checked explicitly)
echo "=== default features ==="
run "cargo check  [default]" "$LOG_DIR/check-default.log" cargo check --all-targets
run "cargo build  [default]" "$LOG_DIR/build-default.log" cargo build
run "cargo test   [default]" "$LOG_DIR/test-default.log" cargo test -- --test-threads=1

# release profile exercises `panic = "abort"` and the optimiser
echo "=== default features, release profile ==="
run "cargo build  [release]" "$LOG_DIR/build-release.log" cargo build --release
run "cargo test   [release]" "$LOG_DIR/test-release.log" cargo test --release -- --test-threads=1

# ---------------------------------------------------------------------------
# 3. the C side has no cmake option(), but CMAKE_BUILD_TYPE still changes the
#    optimisation level, and the C relies on wrap-around signed arithmetic that
#    an optimiser is in principle allowed to exploit.  Re-run the whole suite
#    against each build type (built OUTSIDE c_src/, which is never modified).
# ---------------------------------------------------------------------------
for bt in Release RelWithDebInfo MinSizeRel Debug; do
    echo "=== C reference built with CMAKE_BUILD_TYPE=$bt ==="
    dir="target/c_$bt"
    if cmake -S c_src -B "$dir" "-DCMAKE_BUILD_TYPE=$bt" \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON > "$LOG_DIR/cmake-$bt.log" 2>&1 \
       && cmake --build "$dir" >> "$LOG_DIR/cmake-$bt.log" 2>&1; then
        export CDIFF_C_SO="$PWD/$dir/libtranslated_rust.so"
        run "cargo test   [C:$bt / rust:dev]" "$LOG_DIR/test-c-$bt.log" \
            cargo test -- --test-threads=1
        # the optimised Rust build is where an elided allocation showed up, so
        # cross it with every optimised C build too
        run "cargo test   [C:$bt / rust:release]" "$LOG_DIR/test-c-$bt-rel.log" \
            cargo test --release -- --test-threads=1
        unset CDIFF_C_SO
    else
        echo "  FAIL  cmake $bt (log: $LOG_DIR/cmake-$bt.log)"
        status=1
    fi
done

if [ $status -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS PASSED"
else
    echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $status
