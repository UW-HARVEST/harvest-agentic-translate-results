#!/usr/bin/env bash
# Negative control for the differential test suite.
#
# A test suite that only ever sees a correct translation proves nothing about
# its own sensitivity.  This script injects a series of small, deliberate bugs
# into src/lib.rs, re-runs the differential suites, and requires that EVERY
# mutant is caught (non-zero exit).  src/lib.rs is restored afterwards.
set -u
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK="$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; }
trap restore EXIT

fail=0
mutate() { # <name> <sed-expression>
    local name="$1"; shift
    cp "$BAK" "$SRC"
    if ! sed -i "$1" "$SRC"; then echo "MUTATION SETUP FAILED: $name"; fail=1; return; fi
    if cmp -s "$BAK" "$SRC"; then
        echo "NOT APPLIED (pattern did not match): $name"; fail=1; return
    fi
    cargo build --offline -q 2>/dev/null
    out=$(cargo test --offline -q 2>&1)
    if [ $? -eq 0 ]; then
        echo "SURVIVED (test suite is blind to it): $name"
        fail=1
    else
        caught=$(printf '%s\n' "$out" | grep -c '^FAIL')
        echo "caught  : $name  ($caught failing row(s))"
    fi
}

echo "=== mutation / negative-control run ==="
mutate "start bound uses >= instead of > (rejects the valid start == len)" \
       's|if (start as isize as usize) > len {|if (start as isize as usize) >= len {|'
mutate "stop <= start relaxed to stop < start (allows the empty slice)" \
       's|if s <= start {|if s < start {|'
mutate "stop bound compared as signed (negative stop reaches the wrong branch)" \
       's|if (s as isize as usize) > len {|if s > len as c_int {|'
mutate "slice width off by one" \
       's|stop.wrapping_sub(start),|stop.wrapping_sub(start).wrapping_add(1),|'
mutate "one byte of an error message changed" \
       's|Error: start is off the end|Error: start is Off the end|'
mutate "default stop uses len-1 instead of len" \
       's|stop = len as c_int;|stop = (len as c_int).wrapping_sub(1);|'
mutate "rejection sentinel changed from 1 to 2" \
       's|return 1;|return 2;|g'
# This one is invisible for every string shorter than INT_MAX and therefore
# demonstrates that the 2 GiB row in tests/huge_string.rs earns its keep:
# saturating the size_t -> int conversion instead of truncating it.
mutate "default stop saturates instead of truncating (only observable at len > INT_MAX)" \
       's|stop = len as c_int;|stop = len.min(i32::MAX as usize) as c_int;|'

restore
trap - EXIT
cargo build --offline -q 2>/dev/null

if [ "$fail" -ne 0 ]; then
    echo "=== NEGATIVE CONTROL FAILED: at least one mutant survived ==="
    exit 1
fi
echo "=== all mutants caught; src/lib.rs restored ==="
cargo test --offline -q >/dev/null 2>&1 && echo "restored tree still passes" || { echo "restored tree FAILS"; exit 1; }
