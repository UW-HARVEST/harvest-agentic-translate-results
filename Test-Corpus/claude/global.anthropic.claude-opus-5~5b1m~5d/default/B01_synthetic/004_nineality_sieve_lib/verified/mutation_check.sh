#!/usr/bin/env bash
# Validates that the differential suite has real detection power: inject a
# plausible mistranslation into src/lib.rs, confirm the suite FAILS, restore.
#
# A suite that passes is only meaningful if it would also fail when the Rust
# diverges from the C, so this is run as part of verification.
set -uo pipefail
cd "$(dirname "$0")"

BACKUP=".lib.rs.orig"
cp src/lib.rs "$BACKUP"
restore() { cp "$BACKUP" src/lib.rs; cargo --offline build -q 2>/dev/null; }
trap restore EXIT

mutate() { python3 -c "
import sys
s=open('src/lib.rs').read()
old,new=sys.argv[1],sys.argv[2]
assert old in s, 'pattern not found: '+old
open('src/lib.rs','w').write(s.replace(old,new,1))
" "$1" "$2"; }

overall=0
check() {
    local name="$1" old="$2" new="$3"
    cp "$BACKUP" src/lib.rs
    mutate "$old" "$new" || { echo "SKIP $name (pattern missing)"; return; }
    if ! cargo --offline build -q 2>/dev/null; then
        echo "SKIP       $name  (mutant does not compile -- not a valid mutant)"
        cp "$BACKUP" src/lib.rs
        return
    fi
    local out rc failed
    out=$(timeout 400 cargo --offline test --tests -- --test-threads=1 2>&1)
    rc=$?
    failed=$(printf '%s' "$out" | grep -cE '^test .* FAILED$')
    # The suite "detects" the mutant iff it does not pass: an assertion failure,
    # a watchdog abort on a non-terminating loop, or a timeout all count.
    if [[ $rc -ne 0 ]]; then
        local why="$failed assertion failure(s)"
        [[ "$failed" -eq 0 ]] && why="non-zero exit (watchdog/abort/timeout)"
        if printf '%s' "$out" | grep -q 'WATCHDOG'; then
            why="$why + watchdog fired"
        fi
        echo "DETECTED   $name  [$why]"
    else
        echo "*** MISSED $name  -- the suite cannot detect this divergence!"
        overall=1
    fi
    cp "$BACKUP" src/lib.rs
}

echo "mutation detection check (each mutant applied to a pristine lib.rs)"
echo "------------------------------------------------------------------"
check "Euclidean modulo instead of C truncated modulo" \
      'if val % 10 == 9 {' 'if val.rem_euclid(10) == 9 {'
check "abs() before the modulo test" \
      'if val % 10 == 9 {' 'if val.abs() % 10 == 9 {'
check "checked/panicking increment instead of wrapping" \
      'val = val.wrapping_add(1);' 'val = val + 1;'
check "saturating increment instead of wrapping" \
      'val = val.wrapping_add(1);' 'val = val.saturating_add(1);'
check "off-by-one: compare against 8 instead of 9" \
      'if val % 10 == 9 {' 'if val % 10 == 8 {'
check "print before/after swapped (break before print)" \
      'printf(b"%d\n\0".as_ptr() as *const std::ffi::c_char, val);' \
      'if val % 10 == 9 { return; } printf(b"%d\n\0".as_ptr() as *const std::ffi::c_char, val);'
check "wrong format: no newline" \
      'b"%d\n\0"' 'b"%d \0"'
check "wrong format: unsigned %u instead of %d" \
      'b"%d\n\0"' 'b"%u\n\0"'
check "increment by 2" \
      'val = val.wrapping_add(1);' 'val = val.wrapping_add(2);'
check "off-by-one start value (pre-increment)" \
      'let mut val = val;' 'let mut val = val.wrapping_add(1);'
check "modulo 100 instead of modulo 10" \
      'if val % 10 == 9 {' 'if val % 100 == 9 {'
check "test the condition on the already-incremented value" \
      'if val % 10 == 9 {' 'if val.wrapping_add(1) % 10 == 9 {'

echo "------------------------------------------------------------------"
if [[ $overall -eq 0 ]]; then
    echo "ALL MUTANTS DETECTED"
else
    echo "SOME MUTANTS SURVIVED"
fi
exit $overall
