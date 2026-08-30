#!/usr/bin/env bash
# Harness self-validation: inject deliberate bugs into the Rust translation and
# confirm the differential suite CATCHES them.  A green suite is meaningless if
# it cannot go red.
#
# Each mutant is labelled with its expectation:
#   detect     - the suite MUST fail; if it passes, the harness has a real gap.
#   equivalent - the mutant provably does not change observable behaviour, so the
#                suite MUST still pass (surviving is the correct outcome, and
#                being *detected* would mean the suite is flaky).
#
# Usage: scripts/mutation_check.sh
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

SRC=src/lib.rs
BACKUP=$(mktemp)
cp "$SRC" "$BACKUP"
restore() {
    cp "$BACKUP" "$SRC"
    rm -f "$BACKUP"
    cargo build --release --offline >/dev/null 2>&1
}
trap restore EXIT

fail=0

# run_mutant <expectation> <name> <perl-expr> [test-filter] [profile]
#
# `profile` defaults to `release`; pass `dev` for mutants whose effect is
# profile-sensitive (e.g. rustc's debug-only null-pointer checks).
run_mutant() {
    local expect="$1" name="$2" expr="$3" filter="${4:-}" profile="${5:-release}"
    local pflag=--release
    [ "$profile" = dev ] && pflag=""
    cp "$BACKUP" "$SRC"
    perl -0pi -e "$expr" "$SRC"
    if cmp -s "$BACKUP" "$SRC"; then
        echo "[$expect] $name: PATTERN DID NOT APPLY  <-- fix the mutant"
        fail=1
        return
    fi
    # shellcheck disable=SC2086
    if ! cargo build $pflag --offline >/dev/null 2>&1; then
        echo "[$expect] $name: does not compile, skipped"
        return
    fi
    local detected=1
    # shellcheck disable=SC2086
    if timeout 600 cargo test $pflag --offline ${filter:+"$filter"} >/dev/null 2>&1; then
        detected=0
    fi
    [ "$profile" = dev ] && name="$name [dev profile]"
    if [ "$expect" = detect ]; then
        if [ "$detected" -eq 1 ]; then
            echo "[detect]     $name: detected (good)"
        else
            echo "[detect]     $name: NOT DETECTED  <-- HARNESS GAP"
            fail=1
        fi
    else
        if [ "$detected" -eq 0 ]; then
            echo "[equivalent] $name: survived, as expected (good)"
        else
            echo "[equivalent] $name: unexpectedly detected  <-- suite may be flaky"
            fail=1
        fi
    fi
}

# --- fma_array ---------------------------------------------------------------
run_mutant detect "fma: mul -> add"               's/m1\.wrapping_mul\(m2\)/m1.wrapping_add(m2)/'
run_mutant detect "fma: drop the addend"          's/\.wrapping_add\(a\),/,/'
run_mutant detect "fma: saturating arithmetic"    's/wrapping_mul/saturating_mul/'
run_mutant detect "fma: off-by-one loop bound"    's/while i < len \{/while i < len - 1 {/'
run_mutant detect "fma: len<=0 becomes len<0"     's/while i < len \{/while i <= len {/'
run_mutant detect "fma: iterate in reverse"       's/let mut i: c_int = 0;\n    while i < len \{/let mut i: c_int = len - 1;\n    while i >= 0 {/; s/        i \+= 1;\n    \}/        i -= 1;\n    }/'
run_mutant detect "fma: add the addend twice"     's/m1\.wrapping_mul\(m2\)\.wrapping_add\(a\),/m1.wrapping_mul(m2).wrapping_add(a).wrapping_add(a),/'
run_mutant detect "fma: read mul2 for the addend" 's/let a = core::ptr::read\(add\.wrapping_offset\(idx\)\);/let a = core::ptr::read(mul2.wrapping_offset(idx));/'
run_mutant detect "fma: store at idx+1"          's/out\.wrapping_offset\(idx\),/out.wrapping_offset(idx + 1),/'

# --- call_fma ----------------------------------------------------------------
run_mutant detect "call_fma: len==0 -> 1"         's/if len == 0 \{\n        return 0;/if len == 0 {\n        return 1;/'
run_mutant detect "call_fma: return out[0]"       's/out\[n - 1\]/out[0]/'
run_mutant detect "call_fma: drop the len<0 guard" 's/    if len < 0 \{\n        return 0;\n    \}\n//'
run_mutant detect "call_fma: ones become twos"    's/\n        ones\[i\] = 1;/\n        ones[i] = 2;/'
run_mutant detect "call_fma: zeros become ones"   's/\n        zeros\[i\] = 0;/\n        zeros[i] = 1;/'

# --- driver ------------------------------------------------------------------
run_mutant detect "driver: cap 100 -> 99"         's/while i < 100 \{/while i < 99 {/'
run_mutant detect "driver: cap 100 -> 101"        's/while i < 100 \{/while i < 101 {/'
run_mutant detect "driver: ignore nb advance"     's/cursor\.wrapping_add\(nb\)/cursor.wrapping_add(1)/'
run_mutant detect "driver: accept matched != 1"   's/if matched != 1 \{/if matched == -12345 {/'
run_mutant detect "driver: reject matched == 1"   's/if matched != 1 \{/if matched == 1 {/'
run_mutant detect "driver: drop the newline"      's/c"%d\\n"/c"%d"/'
run_mutant detect "driver: print %u instead"      's/c"%d\\n"\.as_ptr\(\), result/c"%u\\n".as_ptr(), result/'
run_mutant detect "driver: parse %u instead"      's/c"%d%zn"/c"%u%zn"/'
run_mutant detect "driver: parse %i (octal/hex)"  's/c"%d%zn"/c"%i%zn"/'
run_mutant detect "driver: %n width becomes int"  's/c"%d%zn"/c"%dn"/'
run_mutant detect "driver: print i, not result"   's/printf\(c"%d\\n"\.as_ptr\(\), result\)/printf(c"%d\\n".as_ptr(), i as c_int)/'

# --- mutants that provably cannot be observed --------------------------------
# `ones` is fully overwritten by the following loop, so its initialiser is dead.
run_mutant equivalent "call_fma: dead ones initialiser" \
    's/let mut ones: Vec<c_int> = vec!\[0; n\];/let mut ones: Vec<c_int> = vec![2; n];/'
# `data` is only ever read below index i, and i is capped at 100 by the loop.
run_mutant equivalent "driver: data[100] -> data[128]" \
    's/\[0 as c_int; 100\]/[0 as c_int; 128]/'
# glibc's legacy `sscanf` and `__isoc99_sscanf` differ only for `%a` and
# positional `%n$` specifiers, neither of which appears in "%d%zn".
run_mutant equivalent "sscanf: legacy glibc entry point" \
    's/#\[link_name = "__isoc99_sscanf"\]\n    //'
# `out[0] = 0` is immediately overwritten by fma_array for every len >= 1.
run_mutant equivalent "call_fma: out[0] = 0 is dead" \
    's/    out\[0\] = 0;\n//'

# --- profile-sensitive mutants (must be run under the dev profile) -----------
# Reverting `core::ptr::read`/`write` to raw place expressions re-introduces
# rustc's debug-only null-pointer check, which aborts (SIGABRT) where the C build
# faults (SIGSEGV).  Invisible in release, caught by ERRORS.md rows 19-20 in dev.
run_mutant detect "fma: raw place deref instead of ptr::read" \
    's/let m1 = core::ptr::read\(mul1\.wrapping_offset\(idx\)\);/let m1 = *mul1.wrapping_offset(idx);/' \
    err20 dev
run_mutant detect "fma: raw place store instead of ptr::write" \
    's/core::ptr::write\(\n                out\.wrapping_offset\(idx\),\n                m1\.wrapping_mul\(m2\)\.wrapping_add\(a\),\n            \);/*out.wrapping_offset(idx) = m1.wrapping_mul(m2).wrapping_add(a);/' \
    "" dev

restore
trap - EXIT
echo
if [ "$fail" -eq 0 ]; then
    echo "mutation_check: PASS — every 'detect' mutant was caught and every"
    echo "                'equivalent' mutant correctly survived."
else
    echo "mutation_check: FAIL (see above)"
fi
exit "$fail"
