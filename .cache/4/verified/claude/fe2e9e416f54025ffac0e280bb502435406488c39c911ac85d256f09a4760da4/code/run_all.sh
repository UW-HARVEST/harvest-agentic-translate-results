#!/usr/bin/env bash
# Full verification sweep: builds the C .so, then for EVERY feature combination
# and both cargo profiles rebuilds the Rust cdylib, checks symbol parity, and
# runs the Phase B + Phase C differential suites.
#
# `Cargo.toml` declares no optional features (the C build has no configuration
# axes — see CONFIGS.md), so the complete combination list is the empty set,
# exercised both as `--no-default-features` and as the plain default.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
FAILED=0
CARGO_FLAGS="--offline"

step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p c_src/build || exit 1
( cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { fail "C build"; exit 1; }
find c_src/build -maxdepth 1 -name '*.so' -printf '  built %p\n'

# ---------------------------------------------------------------------------
# Every valid feature combination (see CONFIGS.md). The empty combination is
# the only one; it is spelled two ways to cover both cargo resolutions.
FEATURE_COMBOS=(
    "--no-default-features"
    ""                       # plain default
)

for PROFILE in dev release; do
    if [ "$PROFILE" = dev ]; then TARGET_DIR=debug; PROFILE_FLAG=""; \
    else TARGET_DIR=release; PROFILE_FLAG="--release"; fi

    for COMBO in "${FEATURE_COMBOS[@]}"; do
        LABEL="profile=$PROFILE features=[${COMBO:-default}]"

        step "cargo check — $LABEL"
        # shellcheck disable=SC2086
        timeout 600 cargo check $CARGO_FLAGS $PROFILE_FLAG $COMBO --all-targets \
            || fail "cargo check ($LABEL)"

        step "cargo build cdylib — $LABEL"
        # Must precede `cargo test`: cargo test does NOT refresh the cdylib
        # artefact, and the tests dlopen it. (tests/common/mod.rs also guards
        # against a stale artefact.)
        # shellcheck disable=SC2086
        timeout 600 cargo build $CARGO_FLAGS $PROFILE_FLAG $COMBO \
            || { fail "cargo build ($LABEL)"; continue; }

        step "symbol parity — $LABEL"
        ./check_symbols.sh "$TARGET_DIR" || fail "symbol parity ($LABEL)"

        step "Phase B + Phase C differential tests — $LABEL"
        # shellcheck disable=SC2086
        timeout 600 cargo test $CARGO_FLAGS $PROFILE_FLAG $COMBO -- --test-threads=4 \
            || fail "differential tests ($LABEL)"
    done
done

# ---------------------------------------------------------------------------
printf '\n'
if [ "$FAILED" -eq 0 ]; then
    printf '\033[32m===== ALL CONFIGURATIONS PASSED =====\033[0m\n'
else
    printf '\033[31m===== SOME CONFIGURATIONS FAILED =====\033[0m\n'
fi
exit "$FAILED"
