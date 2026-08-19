#!/usr/bin/env bash
# Negative control for the differential suite: inject a bug into the Rust
# translation and require that the suite FAILS. A suite that passes on a
# deliberately broken translation is vacuous, so this is what establishes that
# the passing runs mean something.
#
# Usage: ./mutation_check.sh            (dev profile)
#        PROFILE_FLAG=--release ./mutation_check.sh
set -u
cd "$(dirname "$0")" || exit 1

PROFILE_FLAG="${PROFILE_FLAG:-}"
LIB=src/lib.rs
BAK=$(mktemp)
cp "$LIB" "$BAK"
restore() { cp "$BAK" "$LIB"; }
trap restore EXIT

fail=0
n=0

# mutate <name> <perl-expression-applied-to-whole-file>
mutate() {
  n=$((n + 1))
  restore
  perl -0777 -pi -e "$2" "$LIB"
  if cmp -s "$LIB" "$BAK"; then
    echo "M$n [$1] NO-OP MUTATION (pattern did not match) -- NOT VALIDATED"
    fail=1
    return
  fi
  # Guard against a "mutation" that only edited a comment: require at least one
  # changed line that is not a comment. (A comment-only edit would make the
  # suite look thorough while actually testing nothing.)
  if [ "$(diff "$BAK" "$LIB" | grep -E '^[<>]' \
            | grep -vcE '^[<>][[:space:]]*(//|$)')" -eq 0 ]; then
    echo "M$n [$1] COMMENT-ONLY MUTATION -- NOT VALIDATED"
    fail=1
    return
  fi
  out=$(timeout 600 cargo test --offline --no-default-features $PROFILE_FLAG \
          -- --test-threads=1 2>&1)
  rc=$?
  # "Caught" == the suite did not pass. A nonzero exit covers assertion
  # failures, panics, build errors AND signal deaths (e.g. a mutated
  # printLine(NULL) reaching puts(NULL) segfaults under -O, which is a
  # legitimate way for the suite to reject the mutation).
  if [ "$rc" -ne 0 ]; then
    why=$(echo "$out" | grep -E '^test .* FAILED$' | head -1)
    if [ -z "$why" ]; then
      why=$(echo "$out" | grep -oE 'SIGSEGV|SIGABRT|SIGILL|signal: [0-9]+|error\[E[0-9]+\]' \
              | head -1)
      why="died: ${why:-nonzero exit $rc}"
    fi
    cnt=$(echo "$out" | grep -cE '^test .* FAILED$')
    echo "M$n [$1] CAUGHT (${cnt} failing tests; ${why})"
  else
    echo "M$n [$1] *** NOT CAUGHT -- the suite is blind to this bug ***"
    fail=1
  fi
}

echo "=== mutation / negative-control run (profile: '${PROFILE_FLAG:-dev}') ==="

# 1. Drop the sign extension in printHexCharLine.
mutate "printHexCharLine: no sign extension" \
  's/charHex as c_int/charHex as u8 as c_int/'

# 2. Saturate instead of wrapping the CWE-190 truncation.
mutate "arithmetic: saturating instead of wrapping" \
  's/\(\(data as c_int\) \* 2\) as c_char/((data as c_int) * 2).min(127) as c_char/g'

# 3. driver(): test only the low byte of useGood.
mutate "driver: truthiness on the low byte only" \
  's/if useGood != 0 \{/if (useGood as u8) != 0 {/'

# 4. printLine(): treat NULL like the empty string.
mutate "printLine: NULL emits a newline" \
  's/if !line\.is_null\(\) \{/if true {/'

# 5. goodB2G(): invert the range check so the arithmetic is performed.
mutate "goodB2G: range check inverted" \
  's/if data < \(CHAR_MAX \/ 2\) \{/if data >= (CHAR_MAX \/ 2) {/'

# 6. goodG2B(): wrong constant.
mutate "goodG2B: data = 3 instead of 2" \
  "s/    data = 2;/    data = 3;/"

# 7. goodB2G(): honour the dead store instead of overwriting it.
mutate "goodB2G: dead store not overwritten" \
  "s/    data = b\x27 \x27 as c_char;\n    data = CHAR_MAX;/    data = b\x27 \x27 as c_char;/"

# 8. driver(): swap the two branches.
mutate "driver: branches swapped" \
  's/\{\n        good\(\);\n    \} else \{\n        bad\(\);\n    \}/{\n        bad();\n    } else {\n        good();\n    }/'

# 9. printLine(): drop the trailing newline.
mutate "printLine: no trailing newline" \
  "s/b\"%s\x5cn\x5c0\"/b\"%s\x5c0\"/"

# 10. printHexCharLine(): wrong width specifier.
mutate "printHexCharLine: %x instead of %02x" \
  "s/b\"%02x\x5cn\x5c0\"/b\"%x\x5cn\x5c0\"/"

# 11. Remove the #[no_mangle] export on good() -> symbol parity must catch it.
mutate "good: no_mangle export removed" \
  's/#\[unsafe\(no_mangle\)\]\npub unsafe extern "C" fn good/pub unsafe extern "C" fn good/'

# 12. good(): call the two helpers in the wrong order.
mutate "good: goodG2B/goodB2G order swapped" \
  's/    goodG2B\(\);\n    goodB2G\(\);/    goodB2G();\n    goodG2B();/'

# 13. printHexCharLine(): drop the defensive narrowing of the char parameter.
#     This is the bug the release build originally had; it must stay caught.
mutate "printHexCharLine: defensive narrowing removed" \
  's/    let charHex = charHex as u8 as c_char;\n//'

# 14. bad(): flip the positivity guard so nothing is printed at all.
mutate "bad: positivity guard flipped" \
  's/    let data: c_char;\n    data = CHAR_MAX;\n    if data > 0 \{/    let data: c_char;\n    data = CHAR_MAX;\n    if data < 0 {/'

# 15. printLine(): use the payload as the format string (classic mistranslation
#     of printf("%s\\n", line) -> printf(line)).
mutate "printLine: payload used as the format string" \
  "s/printf\(b\"%s\x5cn\x5c0\"\.as_ptr\(\) as \*const c_char, line\)/printf(line)/"

restore
echo "=== restored; verifying the pristine tree still passes ==="
if timeout 600 cargo test --offline --no-default-features $PROFILE_FLAG \
     -- --test-threads=1 >/dev/null 2>&1; then
  echo "pristine: PASS"
else
  echo "pristine: *** FAIL ***"
  fail=1
fi

echo
if [ "$fail" -eq 0 ]; then
  echo "RESULT: all $n mutations were caught by the differential suite."
else
  echo "RESULT: PROBLEM -- see the NOT CAUGHT / NO-OP lines above."
fi
exit "$fail"
