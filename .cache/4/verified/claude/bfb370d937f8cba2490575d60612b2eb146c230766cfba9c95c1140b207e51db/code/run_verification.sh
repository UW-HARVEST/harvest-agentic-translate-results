#!/usr/bin/env bash
# Full verification sweep: builds the C .so, builds the Rust cdylib for every
# feature combination, diffs the exported symbols, and runs the differential
# test suites (Phase B + Phase C) against every built artifact.
set -uo pipefail
cd "$(dirname "$0")" || exit 1
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------- feature combos
# Cargo.toml has no [features] section, so the only valid combination is the
# empty set; it is exercised both as "default" and as "--no-default-features".
COMBOS=("" "--no-default-features")
if grep -q '^\[features\]' Cargo.toml; then
    echo "WARNING: [features] section appeared in Cargo.toml — extend COMBOS!" >&2
fi

# ---------------------------------------------------------------- 1. C library
step "building the C shared library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) && ok "c_src/build/libdriver.so" || bad "C build"

# ---------------------------------------------------------------- 2. cargo check
step "cargo check for every feature combination"
for c in "${COMBOS[@]}"; do
    if timeout 300 cargo check --offline --all-targets $c >/dev/null 2>&1; then
        ok "cargo check ${c:-<default>}"
    else
        bad "cargo check ${c:-<default>}"
    fi
done

# ---------------------------------------------------------------- 3. build + test
for profile in release debug; do
    for c in "${COMBOS[@]}"; do
        step "profile=$profile features=${c:-<default>}"
        relflag=""; [ "$profile" = release ] && relflag="--release"
        if ! timeout 300 cargo build --offline $relflag $c >/dev/null 2>&1; then
            bad "cargo build $profile ${c:-<default>}"; continue
        fi
        SO="target/$profile/libdriver.so"
        [ -f "$SO" ] || { bad "missing $SO"; continue; }

        # symbol diff: every C export must be exported by the Rust .so
        DIFF=$(comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
                        <(nm -D --defined-only "$SO"                    | awk '{print $NF}' | sort))
        if [ -n "$DIFF" ]; then
            bad "symbols missing from $SO: $(echo "$DIFF" | tr '\n' ' ')"
        else
            ok "symbol parity for $SO"
        fi

        # differential suites, always loading THIS .so through libloading
        if DRIVER_RUST_SO="$PWD/$SO" timeout 590 cargo test --offline $relflag $c \
             --test symbols --test differential >/dev/null 2>&1; then
            ok "Phase B (differential) + symbols against $SO"
        else
            bad "Phase B (differential)/symbols against $SO"
        fi
        if DRIVER_RUST_SO="$PWD/$SO" timeout 590 cargo test --offline $relflag $c \
             --test error_paths -- --test-threads=1 >/dev/null 2>&1; then
            ok "Phase C (error paths) against $SO"
        else
            bad "Phase C (error paths) against $SO"
        fi
    done
done

step "result"
if [ "$FAIL" = 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit $FAIL
