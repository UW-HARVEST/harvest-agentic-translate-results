#!/usr/bin/env bash
# Phase D driver: enumerate every build configuration, then run the full
# differential suite and the symbol-parity check under each one.
#
# Feature combinations are derived mechanically from Cargo.toml's [features]
# table (and CMakeLists.txt's option()/add_definitions, of which there are
# none), never hand-listed.
set -uo pipefail
cd "$(dirname "$0")"

# Scratch space (honour TMPDIR; /tmp may be read-only).
TMP="${TMPDIR:-/tmp}"
CC_LOG="$TMP/cc.$$"
CT_LOG="$TMP/ct.$$"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate the feature powerset straight out of Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', src, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            k = line.split('=')[0].strip().strip('"')
            if k != 'default':
                feats.append(k)
for f in feats:
    print(f)
PY
)

note "Cargo.toml [features]"
if [ "${#FEATURES[@]}" -eq 0 ]; then
    echo "no [features] table -> the feature powerset is the single empty set"
else
    printf 'features: %s\n' "${FEATURES[*]}"
fi

note "CMakeLists.txt build-time options"
if grep -qE '^\s*(option|add_definitions|target_compile_definitions)' c_src/CMakeLists.txt; then
    grep -nE '^\s*(option|add_definitions|target_compile_definitions)' c_src/CMakeLists.txt
else
    echo "no option()/add_definitions/#ifdef -> the C has a single configuration"
fi

# Build the powerset of feature names (empty set included).
COMBOS=("")
for f in "${FEATURES[@]}"; do
    new=()
    for c in "${COMBOS[@]}"; do
        if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    COMBOS+=("${new[@]}")
done

note "Feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<no features>}'"; done

# ---------------------------------------------------------------------------
# 2. cargo check + full differential suite for every combination and profile
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    label="${combo:-<no features>}"
    for profile in dev release; do
        relflag=""
        [ "$profile" = release ] && relflag="--release"

        note "cargo check --no-default-features --features '$combo' ($profile)"
        if timeout 600 cargo check $relflag --no-default-features \
             --features "$combo" --all-targets >"$CC_LOG" 2>&1; then
            echo "OK"
        else
            echo "FAILED"; tail -30 "$CC_LOG"; fail=1
        fi

        note "cargo test --no-default-features --features '$combo' ($profile)"
        if timeout 600 cargo test $relflag --no-default-features \
             --features "$combo" >"$CT_LOG" 2>&1; then
            grep -E '^test result:' "$CT_LOG"
        else
            echo "FAILED"; grep -E '^test |panicked|DIVERGENCE' "$CT_LOG" | head -40; fail=1
        fi
    done
done

# ---------------------------------------------------------------------------
# 3. Symbol parity: every symbol the C .so exports, the Rust .so must export
# ---------------------------------------------------------------------------
for profile in debug release; do
    cso=$(find "target/$profile/build" -name libc_driver.so 2>/dev/null | head -1)
    rso=$(find "target/$profile/build" -name librust_driver.so 2>/dev/null | head -1)
    [ -n "$cso" ] && [ -n "$rso" ] || continue

    note "nm -D symbol parity ($profile)"
    syms() { nm -D --defined-only "$1" | awk '{print $3}' | grep -vE '^(_init|_fini)$' | sort -u; }
    missing=$(comm -23 <(syms "$cso") <(syms "$rso"))
    extra=$(comm -13 <(syms "$cso") <(syms "$rso"))
    echo "C exports:    $(syms "$cso" | tr '\n' ' ')"
    echo "Rust exports: $(syms "$rso" | tr '\n' ' ')"
    if [ -z "$missing" ]; then
        echo "OK: 0 C symbols missing from the Rust .so"
    else
        echo "FAILED: missing from Rust .so: $missing"; fail=1
    fi
    [ -n "$extra" ] && echo "note: Rust-only symbols: $extra"

    # No unresolved symbols at load time.
    if ldd -r "$rso" 2>&1 | grep -q "undefined symbol"; then
        echo "FAILED: unresolved symbols in the Rust .so"
        ldd -r "$rso" 2>&1 | grep "undefined symbol"; fail=1
    else
        echo "OK: 0 unresolved symbols in the Rust .so"
    fi
done

rm -f "$CC_LOG" "$CT_LOG"
note "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
