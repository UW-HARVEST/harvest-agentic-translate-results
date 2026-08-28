#!/usr/bin/env bash
# Full verification run: builds the C .so and the Rust cdylib, then runs the
# differential suite under every build profile and every feature combination.
#
# IMPORTANT: `cargo test` does NOT build `crate-type = ["cdylib"]` artifacts
# (the integration tests never `use` the crate, so cargo has no reason to link
# it). Every `cargo test` below is therefore preceded by an explicit
# `cargo build` for the same profile/features. The test harness additionally
# refuses to run against a .so older than src/, so a missed rebuild fails loudly
# instead of producing a vacuous green run.
#
# Usage: ./run_all_tests.sh
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(pwd)"

fail=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library (ground truth)
# ---------------------------------------------------------------------------
step "Building C shared library"
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find ../c_src/build -name '*.so' | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
step "Enumerating feature combinations"
FEATURES="$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n and n != 'default':
                names.append(n)
print(' '.join(names))
PY
)"
if [ -z "$FEATURES" ]; then
    echo "No [features] declared -> exactly one feature configuration."
    # Still exercise the flags explicitly: they must be no-ops here.
    COMBOS=("" "--no-default-features" "--all-features")
else
    echo "Features: $FEATURES"
    COMBOS=("" "--no-default-features" "--all-features")
    for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
fi

# ---------------------------------------------------------------------------
# 3. cargo check every combination
# ---------------------------------------------------------------------------
step "cargo check across feature combinations"
for c in "${COMBOS[@]}"; do
    if cargo check -q $c 2>/dev/null; then
        echo "  OK    cargo check ${c:-<default>}"
    else
        echo "  FAIL  cargo check ${c:-<default>}"; fail=1
    fi
done

# ---------------------------------------------------------------------------
# 4. Build + test every (profile x feature-combination)
# ---------------------------------------------------------------------------
for prof in debug release; do
    PF=""; [ "$prof" = release ] && PF="--release"
    for c in "${COMBOS[@]}"; do
        step "profile=$prof features=${c:-<default>}"

        if ! cargo build -q $PF $c 2>&1 | tail -5; then :; fi
        SO="target/$prof/libsiphash_lib.so"
        if [ ! -f "$SO" ]; then
            echo "  FAIL  cdylib missing: $SO"; fail=1; continue
        fi

        # Symbol parity for this exact artifact.
        missing="$(comm -23 \
            <(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBRWVGS]$/ {print $3}' | sort -u) \
            <(nm -D --defined-only "$SO"   | awk '$2 ~ /^[TDBRWVGS]$/ {print $3}' | sort -u))"
        if [ -n "$missing" ]; then
            echo "  FAIL  symbols missing from Rust .so:"; echo "$missing" | sed 's/^/          /'
            fail=1
        else
            echo "  OK    symbol parity (0 missing)"
        fi

        if timeout 600 cargo test -q $PF $c 2>&1 | tail -25; then
            echo "  OK    tests"
        else
            echo "  FAIL  tests"; fail=1
        fi
    done
done

# ---------------------------------------------------------------------------
# 5. Final symbol report
# ---------------------------------------------------------------------------
step "Final symbol diff (C vs Rust release)"
diff <(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBRWVGS]$/ {print $3}' | sort -u) \
     <(nm -D --defined-only target/release/libsiphash_lib.so | awk '$2 ~ /^[TDBRWVGS]$/ {print $3}' | sort -u) \
     && echo "IDENTICAL public symbol sets"

step "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
