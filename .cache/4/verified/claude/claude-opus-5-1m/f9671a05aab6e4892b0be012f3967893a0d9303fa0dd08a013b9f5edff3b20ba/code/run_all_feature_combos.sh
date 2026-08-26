#!/usr/bin/env bash
# Phase D driver: enumerate every build-time configuration and run the whole
# differential suite (Phases B + C + D) under each one.
#
# Two axes exist:
#   1. Cargo features. Enumerated mechanically from Cargo.toml -- this crate
#      declares NO [features] section, so the only combination is the empty set
#      (which is simultaneously "default" and "--no-default-features").
#   2. The optimisation level the Rust cdylib under test is compiled at. This is
#      a real axis: the C reference is built by CMake with no -O flag, and the
#      translated code relies on wrapping arithmetic and on a deliberate stack
#      probe, both of which LLVM treats differently at -O0 vs -O2/-O3.
set -uo pipefail
cd "$(dirname "$0")"

fail=0
run() { # run <label> <env-assignments...> -- <cmd...>
    local label="$1"; shift
    echo "=============================================================="
    echo "### $label"
    echo "=============================================================="
    if "$@"; then
        echo ">>> PASS: $label"
    else
        echo ">>> FAIL: $label"
        fail=1
    fi
}

# ---------------------------------------------------------------------------
# 1. Enumerate cargo feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
echo "declared features: ${#FEATURES[@]} -> ${FEATURES[*]:-<none>}"

# Power set of the declared features (empty set always included).
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
    [ -z "$f" ] && continue
    for existing in "${COMBOS[@]}"; do
        if [ -z "$existing" ]; then COMBOS+=("$f"); else COMBOS+=("$existing,$f"); fi
    done
done
echo "feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. cargo check for every feature combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    label="cargo check --no-default-features --features '${combo}'"
    run "$label" cargo check --offline --no-default-features --features "$combo" --all-targets
done
run "cargo check (default features)" cargo check --offline --all-targets
run "cargo build --release (panic=abort)" cargo build --offline --release

# ---------------------------------------------------------------------------
# 3. Full differential suite for every feature combination x opt-level
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    for opt in 0 2 3; do
        run "cargo test --no-default-features --features '${combo}'  [cdylib opt-level=$opt]" \
            env DRIVER_RUST_OPT="$opt" cargo test --offline --no-default-features \
                --features "$combo" -- --test-threads=4
    done
    # Third axis: rustc's `debug_assertions`. With them on, rustc's MIR null
    # check converts a NULL dereference into a controlled abort instead of
    # SIGSEGV; the suite asserts the right expectation for each setting.
    for opt in 0 3; do
        run "cargo test --features '${combo}'  [cdylib opt-level=$opt, debug-assertions=on]" \
            env DRIVER_RUST_OPT="$opt" DRIVER_RUST_DEBUG_ASSERTIONS=on \
                cargo test --offline --no-default-features --features "$combo" -- --test-threads=4
    done
done

# ---------------------------------------------------------------------------
# 4. The suite against the actual `cargo build --release` artifact
# ---------------------------------------------------------------------------
run "cargo test against target/release/libdriver.so" \
    env DRIVER_RUST_SO="$PWD/target/release/libdriver.so" cargo test --offline -- --test-threads=4

echo
if [ "$fail" -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
