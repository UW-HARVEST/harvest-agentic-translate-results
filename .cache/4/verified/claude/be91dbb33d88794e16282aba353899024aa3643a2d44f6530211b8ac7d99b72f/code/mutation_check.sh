#!/bin/bash
# Harness validation: every mutation of the Rust translation MUST be detected by
# the differential suite (as a test failure, or as a loud timeout for mutations
# that make the loop non-terminating). Restores src/lib.rs on exit.
set -u
cd "$(dirname "$0")"
ORIG="${TMPDIR:-/tmp}/lib.rs.mutation_orig"
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; }
trap restore EXIT

fails=0
ONLY="${ONLY:-}"

mutate() { # name, sed args...
  local name="$1"; shift
  if [ -n "$ONLY" ] && [[ "$name" != *"$ONLY"* ]]; then return; fi
  restore
  sed -i "$@" src/lib.rs
  if diff -q "$ORIG" src/lib.rs >/dev/null; then
    echo "MUTATION '$name': *** SED DID NOT APPLY ***"
    fails=$((fails+1))
    return
  fi
  local out rc
  out=$(timeout 300 cargo test --no-default-features --offline -- --test-threads=1 2>&1)
  rc=$?
  if echo "$out" | grep -qE "cannot build fresh Rust cdylib|error\[E[0-9]+\]|error: could not compile"; then
    echo "MUTATION '$name': COMPILE ERROR (mutation not applicable)"
    return
  fi
  if [ "$rc" -eq 124 ]; then
    echo "MUTATION '$name': CAUGHT (suite timed out - non-terminating divergence)"
    return
  fi
  if echo "$out" | grep -qE "^test result: FAILED"; then
    echo "MUTATION '$name': CAUGHT ($(echo "$out" | grep -cE '^test [a-z0-9_]+ \.\.\. FAILED') failing tests)"
    echo "$out" | grep -oE "^test [a-z0-9_]+ \.\.\. FAILED" | head -4 | sed 's/^/    /'
    return
  fi
  echo "MUTATION '$name': *** NOT CAUGHT *** (harness is blind here)"
  fails=$((fails+1))
}

mutate "floor-mod instead of C truncating %" \
  -e 's/if val % 10 == 9 {/if val.rem_euclid(10) == 9 {/'
mutate "saturating_add instead of wrapping overflow" \
  -e 's/val = val.wrapping_add(1);/val = val.saturating_add(1);/'
mutate "check before print (drops the first line)" \
  -e 's|^    loop {|    loop {\n        if val % 10 == 9 { return; }|'
mutate "print val+1 instead of val (off-by-one output)" \
  -e 's/as \*const c_char, val)/as *const c_char, val.wrapping_add(1))/'
mutate "format string without newline" \
  -e 's/b"%d\\n\\0"/b"%d \\0"/'
mutate "off-by-one terminator (8 instead of 9)" \
  -e 's/if val % 10 == 9 {/if val % 10 == 8 {/'
mutate "step by 2" \
  -e 's/val.wrapping_add(1)/val.wrapping_add(2)/'
mutate "unsigned printing (%u)" \
  -e 's/b"%d\\n\\0"/b"%u\\n\\0"/'
mutate "long printing (%ld)" \
  -e 's/b"%d\\n\\0"/b"%ld\\n\\0"/'
mutate "wrong export name (no_mangle symbol renamed)" \
  -e 's/pub extern "C" fn sieve(/pub extern "C" fn sieve_(/'

restore
timeout 300 cargo build --no-default-features --offline >/dev/null 2>&1
echo "-------------------------------------------"
if [ "$fails" -eq 0 ]; then
  echo "ALL MUTATIONS DETECTED - harness is sensitive"
else
  echo "$fails MUTATION(S) NOT DETECTED"
fi
exit "$fails"
