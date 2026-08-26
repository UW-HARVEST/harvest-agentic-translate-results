#!/usr/bin/env bash
# Phase A + Phase D: enumerate every build-time configuration and run the whole
# differential suite under each one.
#
# The feature list is read out of Cargo.toml rather than hard-coded, so adding a
# feature automatically extends the matrix. `pinflate` currently declares no
# `[features]` table and `c_src/CMakeLists.txt` declares no `option()` /
# `target_compile_definitions` / `CMAKE_BUILD_TYPE`, so the matrix is the single
# default (empty) combination -- which this script proves rather than assumes.
set -u
cd "$(dirname "$0")/.." || exit 1
ulimit -c 0

echo "=== Cargo.toml [features] ==="
FEATFILE="${TMPDIR:-/tmp}/.pinflate_features"
python3 - "$FEATFILE" <<'PY'
import re, sys
featfile = sys.argv[1]
src = open("Cargo.toml").read()
m = re.search(r"^\[features\]\s*$(.*?)(^\[|\Z)", src, re.M | re.S)
if not m:
    print("(no [features] table)")
    open(featfile, "w").close()
    sys.exit(0)
feats = [l.split("=")[0].strip() for l in m.group(1).splitlines()
         if l.strip() and not l.strip().startswith("#")]
print("features:", feats)
open(featfile, "w").write("\n".join(feats))
PY

echo
echo "=== c_src/CMakeLists.txt build-time configuration ==="
if grep -Eq 'option\(|add_definitions|target_compile_definitions|CMAKE_BUILD_TYPE' c_src/CMakeLists.txt; then
    echo "!! the C build has configuration knobs -- extend this script"
    grep -En 'option\(|add_definitions|target_compile_definitions|CMAKE_BUILD_TYPE' c_src/CMakeLists.txt
    exit 1
else
    echo "(none: single configuration; no CMAKE_BUILD_TYPE, so assert() is live)"
fi

FEATS=$(cat "$FEATFILE" 2>/dev/null || true)

# Build the list of combinations: the powerset of the declared features, or just
# the empty combination when there are none.
combos=()
if [ -z "$FEATS" ]; then
    combos=("")
else
    mapfile -t arr <<<"$FEATS"
    n=${#arr[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        c=""
        for ((b = 0; b < n; b++)); do
            if (((mask >> b) & 1)); then
                c="${c:+$c,}${arr[b]}"
            fi
        done
        combos+=("$c")
    done
fi

echo
echo "=== ${#combos[@]} feature combination(s) to verify ==="

rc=0
for c in "${combos[@]}"; do
    label="${c:-<default/no features>}"
    echo
    echo "--- cargo check --no-default-features --features '$c' ---"
    if [ -z "$c" ]; then
        timeout 600 cargo check --offline --no-default-features --all-targets 2>&1 | tail -3
        st=${PIPESTATUS[0]}
    else
        timeout 600 cargo check --offline --no-default-features --features "$c" --all-targets 2>&1 | tail -3
        st=${PIPESTATUS[0]}
    fi
    [ "$st" -ne 0 ] && { echo "CHECK FAILED for $label"; rc=1; continue; }

    echo "--- cargo test --no-default-features --features '$c' ---"
    rm -rf target/cdylib_build
    if [ -z "$c" ]; then
        out=$(timeout 600 cargo test --offline --no-default-features 2>&1)
    else
        out=$(timeout 600 cargo test --offline --no-default-features --features "$c" 2>&1)
    fi
    echo "$out" | grep -E "^test result:" | sed 's/^/    /'
    if echo "$out" | grep -qE "test result: FAILED|error: could not compile"; then
        echo "TEST FAILED for $label"
        echo "$out" | grep -E "^test .* FAILED|diverged|^error" | head -10 | sed 's/^/    /'
        rc=1
    else
        echo "    OK for $label"
    fi
done

echo
if [ "$rc" -eq 0 ]; then
    echo "=== all ${#combos[@]} feature combination(s) PASSED ==="
else
    echo "=== FAILURES present ==="
fi
exit "$rc"
