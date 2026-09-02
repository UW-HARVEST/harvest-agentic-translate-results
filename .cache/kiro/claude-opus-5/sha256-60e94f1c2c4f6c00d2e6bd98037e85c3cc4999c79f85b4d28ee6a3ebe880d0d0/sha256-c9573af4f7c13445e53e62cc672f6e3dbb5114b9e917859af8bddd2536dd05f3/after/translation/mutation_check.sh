#!/usr/bin/env bash
# Suite-validation (anti-vacuous-pass) check.
#
# A differential suite that passes is only meaningful if it would FAIL on a
# wrong translation. This script injects known bugs into src/lib.rs, rebuilds,
# and asserts the suite rejects each one. src/lib.rs is always restored.
#
# It exists because the suite DID once pass vacuously: `cargo test` does not
# rebuild the cdylib (nothing links it), so every mutation silently tested the
# previously built .so. The harness now has a staleness guard, and this script
# rebuilds explicitly.

set -uo pipefail
cd "$(dirname "$0")" || exit 1

ORIG=$(mktemp); cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; rm -f "$ORIG"; timeout 600 cargo build --release >/dev/null 2>&1; }
trap restore EXIT

FAIL=0
mutate() {
  local name="$1" old="$2" new="$3"
  cp "$ORIG" src/lib.rs
  python3 -c "
import sys
p='src/lib.rs'; s=open(p).read()
old,new=sys.argv[1],sys.argv[2]
if old not in s: sys.exit(3)
open(p,'w').write(s.replace(old,new,1))
" "$old" "$new"
  case $? in
    3) echo "  [$name] SKIP: pattern no longer present in src/lib.rs"; return;;
    0) ;;
    *) echo "  [$name] SKIP: mutation script error"; return;;
  esac

  if ! timeout 600 cargo build --release >/dev/null 2>&1; then
    echo "  [$name] SKIP: mutant does not compile"; return
  fi

  timeout 600 cargo test --release --test differential -- --test-threads=1 \
    >/tmp/mut_$$.log 2>&1
  local rc=$?
  if (( rc == 0 )); then
    echo "  [$name] NOT DETECTED -- the suite has a blind spot here"
    FAIL=1
  else
    local n
    n=$(awk '/^failures:$/{f=1;next} f&&/^    /{print $1}' /tmp/mut_$$.log | sort -u | wc -l)
    if (( n > 0 )); then
      echo "  [$name] detected by $n test(s)"
    else
      echo "  [$name] detected (library crashed / non-zero exit: rc=$rc)"
    fi
  fi
}

echo "=== suite validation: every mutant must be REJECTED ==="

mutate "driver: truncate selector to low byte" \
  'if useGood != 0 {' 'if (useGood as u8) != 0 {'

mutate "goodG2B: wrong constant (2 -> 3)" \
  'data = 2;' 'data = 3;'

mutate "printLine: drop the NULL check" \
  'if !line.is_null() {' 'if true {'

mutate "printHexCharLine: drop the low-byte narrowing" \
  'let charHex = charHex as c_char;
    printf' 'printf'

mutate "goodB2G: widen the range check" \
  'if data < (CHAR_MAX / 2) {' 'if data <= CHAR_MAX {'

mutate "printLine: add UTF-8 validation the C does not do" \
  'printf(b"%s\n\0".as_ptr() as *const c_char, line);' \
  'if std::ffi::CStr::from_ptr(line).to_str().is_ok() { printf(b"%s\n\0".as_ptr() as *const c_char, line); }'

mutate "bad: saturating instead of wrapping multiply" \
  'data.wrapping_mul(2)' 'data.saturating_mul(2)'

mutate "driver: invert the selector" \
  'if useGood != 0 {
        good();
    } else {
        bad();
    }' 'if useGood != 0 {
        bad();
    } else {
        good();
    }'

mutate "good: swap goodG2B / goodB2G order" \
  'goodG2B();
    goodB2G();' 'goodB2G();
    goodG2B();'

rm -f /tmp/mut_$$.log
echo "=== RESULT ==="
if (( FAIL )); then echo "SUITE VALIDATION: FAILED (blind spot found)"; exit 1; fi
echo "SUITE VALIDATION: every mutant rejected"
