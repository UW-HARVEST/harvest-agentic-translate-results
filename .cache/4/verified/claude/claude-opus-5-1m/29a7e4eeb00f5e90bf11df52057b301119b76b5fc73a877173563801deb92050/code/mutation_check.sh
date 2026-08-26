#!/usr/bin/env bash
# Non-vacuity check for the differential suite.
#
# Injects realistic translation bugs into src/lib.rs one at a time and requires
# the suite to FAIL for each. A suite that passes a mutated translation proves
# nothing, so this is what makes the green run meaningful.
#
# src/lib.rs is always restored, including on interrupt.
set -uo pipefail
cd "$(dirname "$0")"

LIB=src/lib.rs
BAK=$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")
cp "$LIB" "$BAK"
restore() { cp "$BAK" "$LIB"; rm -f "$BAK"; }
trap restore EXIT INT TERM

rc=0
mutate() { # mutate <name> <python-expression-file-transform>
  local name="$1" py="$2"
  cp "$BAK" "$LIB"
  python3 - "$LIB" <<PY
import sys
p = sys.argv[1]
s = open(p).read()
$py
open(p, 'w').write(s)
PY
  if diff -q "$BAK" "$LIB" >/dev/null; then
    echo "!! MUTATION '$name' DID NOT APPLY (pattern not found) -- check the script"
    rc=1
    return
  fi
  if timeout 600 cargo test --offline >/dev/null 2>&1; then
    echo "!! NOT DETECTED: '$name' -- the suite passes a knowingly-wrong translation"
    rc=1
  else
    echo "   detected: $name"
  fi
}

echo "mutation detection check (each mutation MUST be detected):"

mutate "drop the NULL guard in printLine" \
  "s = s.replace('if !line.is_null() {', 'if true {')"

mutate "invert the NULL guard in printLine" \
  "s = s.replace('if !line.is_null() {', 'if line.is_null() {')"

mutate "bad() also calls the dead helperBad()" \
  "s = s.replace('printLine(c\"bad()\".as_ptr());\n}', 'printLine(c\"bad()\".as_ptr());\n    helper_bad();\n}')"

mutate "good() stops calling helperGood()" \
  "s = s.replace('    helper_good();\n', '')"

mutate "printLine loses the trailing newline" \
  "s = s.replace('c\"%s\\\\n\".as_ptr()', 'c\"%s\".as_ptr()')"

mutate "typo in a driver() banner string" \
  "s = s.replace('c\"Finished good()\"', 'c\"Finished good\"')"

mutate "driver() reorders good()/bad()" \
  "s = s.replace('''    printLine(c\"Calling good()...\".as_ptr());
    good();
    printLine(c\"Finished good()\".as_ptr());
    printLine(c\"Calling bad()...\".as_ptr());
    bad();
    printLine(c\"Finished bad()\".as_ptr());''', '''    printLine(c\"Calling bad()...\".as_ptr());
    bad();
    printLine(c\"Finished bad()\".as_ptr());
    printLine(c\"Calling good()...\".as_ptr());
    good();
    printLine(c\"Finished good()\".as_ptr());''')"

mutate "printLine goes through Rust's lossy UTF-8 println! instead of printf" \
  "s = s.replace('        printf(c\"%s\\\\n\".as_ptr(), line);',
                 '        println!(\"{}\", std::ffi::CStr::from_ptr(line).to_string_lossy());')"

mutate "printLine treats the payload as the printf format string" \
  "s = s.replace('printf(c\"%s\\\\n\".as_ptr(), line);', 'printf(line);')"

echo
if [[ $rc -eq 0 ]]; then
  echo "ALL MUTATIONS DETECTED -- the differential suite is non-vacuous"
else
  echo "SOME MUTATIONS WENT UNDETECTED -- test coverage gap"
fi
exit $rc
