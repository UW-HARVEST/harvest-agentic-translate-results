#!/usr/bin/env bash
# Mutation-sensitivity check for the differential suite.
#
# Each mutation injects a subtle bug into the Rust translation and confirms the
# suite CATCHES it. A surviving mutation means the suite has a blind spot —
# UNLESS the mutation is provably semantically equivalent to the original, in
# which case it is listed as an EXPECTED survivor with a proof (see below).
#
# Run from translation/. Exit code = number of unexpected survivors.
set -uo pipefail
cd "$(dirname "$0")"

BAK=$(mktemp)
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; cargo build --release >/dev/null 2>&1; }
trap restore EXIT

unexpected=0
caught=0
expected_survivors=0

# run_mutation <expect: kill|equivalent> <name> <from1> <to1> [<from2> <to2>]
run_mutation() {
  local expect="$1" name="$2"; shift 2
  cp "$BAK" src/lib.rs
  if ! MUT_ARGS_COUNT=$# python3 - "$@" <<'PY'
import os, sys
args = sys.argv[1:]
s = open('src/lib.rs').read()
for i in range(0, len(args), 2):
    frm = args[i].replace('\\n', '\n')
    to  = args[i+1].replace('\\n', '\n')
    if frm not in s:
        sys.stderr.write("pattern not found: %r\n" % frm)
        sys.exit(9)
    s = s.replace(frm, to, 1)
open('src/lib.rs', 'w').write(s)
PY
  then echo "ERROR $name (pattern not found)"; unexpected=$((unexpected+1)); return; fi

  if ! cargo build --release >/dev/null 2>&1; then
    echo "ERROR $name (mutant does not compile)"; unexpected=$((unexpected+1)); return
  fi

  if timeout 600 cargo test >/dev/null 2>&1; then
    if [ "$expect" = equivalent ]; then
      echo "survived (expected, provably equivalent)  $name"
      expected_survivors=$((expected_survivors+1))
    else
      echo "SURVIVED  $name   <-- BLIND SPOT"
      unexpected=$((unexpected+1))
    fi
  else
    if [ "$expect" = equivalent ]; then
      echo "KILLED but expected to survive  $name   <-- equivalence proof is wrong"
      unexpected=$((unexpected+1))
    else
      echo "caught    $name"
      caught=$((caught+1))
    fi
  fi
}

echo "### saturation boundaries (ERRORS.md E7/E8)"
# EQUIVALENT: `>=` and `>` differ only at number == 2147483647.0 exactly. There
# the else branch computes (int)2147483647.0 == INT_MAX, i.e. the same value the
# saturating branch writes. Observationally identical.
run_mutation equivalent "E7: >= INT_MAX  ->  > INT_MAX" \
  'if number >= INT_MAX as c_double {' 'if number > INT_MAX as c_double {'
# EQUIVALENT: same argument at number == -2147483648.0, where
# (int)(-2147483648.0) == INT_MIN.
run_mutation equivalent "E8: <= INT_MIN  ->  < INT_MIN" \
  '} else if number <= INT_MIN as c_double {' '} else if number < INT_MIN as c_double {'
run_mutation kill "E7: saturate to INT_MIN instead of INT_MAX" \
  '(*item).valueint = INT_MAX;' '(*item).valueint = INT_MIN;'
run_mutation kill "E8: saturate to INT_MAX instead of INT_MIN" \
  '(*item).valueint = INT_MIN;' '(*item).valueint = INT_MAX;'
run_mutation kill "E7: >= INT_MAX  ->  >= INT_MAX - 1" \
  'if number >= INT_MAX as c_double {' 'if number >= (INT_MAX - 1) as c_double {'
run_mutation kill "E8: <= INT_MIN  ->  <= INT_MIN + 1" \
  '} else if number <= INT_MIN as c_double {' '} else if number <= (INT_MIN + 1) as c_double {'

echo
echo "### scan alphabet (CONFIGS C18, ERRORS E4/E6)"
run_mutation kill "scan: drop 'E'" \
  "| b'e' | b'E' => {" "| b'e' => {"
run_mutation kill "scan: drop 'e'" \
  "| b'e' | b'E' => {" "| b'E' => {"
run_mutation kill "scan: drop '+'" "b'9' | b'+' | b'-'" "b'9' | b'-'"
run_mutation kill "scan: drop '-'" "b'9' | b'+' | b'-'" "b'9' | b'+'"
run_mutation kill "scan: drop '.' (route it to an unreachable byte)" \
  "            b'.' => {" "            b'#' => {"
run_mutation kill "scan: accept '.' but do not count it" \
  "                number_string_length += 1;\\n                has_decimal_point" \
  "                number_string_length += 0;\\n                has_decimal_point"
run_mutation kill "scan: also accept 'x'" \
  '_ => break, /* goto loop_end */' \
  "b'x' => { number_string_length += 1; } _ => break,"
run_mutation kill "scan: also accept whitespace" \
  '_ => break, /* goto loop_end */' \
  "b' ' => { number_string_length += 1; } _ => break,"
# EQUIVALENT: the rewrite loop replaces '.' with `decimal_point`, and
# `decimal_point` is the constant b'.', so the loop is a no-op. Never entering it
# is observationally identical. (This is the C quirk the header comment hints at:
# the "localise the decimal separator" step was never wired to the locale.)
run_mutation equivalent "'.': never set has_decimal_point (rewrite loop is a no-op)" \
  'has_decimal_point = CJSON_TRUE;' 'has_decimal_point = CJSON_FALSE;'
run_mutation kill "rewrite loop: use ',' as the decimal point" \
  "let decimal_point: c_uchar = b'.';" "let decimal_point: c_uchar = b',';"

echo
echo "### bound check / can_access_at_index (ERRORS.md E5)"
run_mutation kill "bound: < length  ->  <= length" \
  'buffer.offset.wrapping_add(index) < buffer.length' \
  'buffer.offset.wrapping_add(index) <= buffer.length'
run_mutation kill "bound: ignore offset" \
  'buffer.offset.wrapping_add(index) < buffer.length' \
  'index < buffer.length'
# EQUIVALENT: `offset + index` can never overflow while the loop is running.
# By induction: the loop reaches iteration i only if offset + (i-1) < length <=
# SIZE_MAX, hence offset + (i-1) <= SIZE_MAX - 1, hence offset + i <= SIZE_MAX.
# At i == 0 the sum is just `offset`. So the addition is always exact and
# wrapping/saturating/checked semantics coincide. (The C relies on the same
# fact; `wrapping_add` is kept because it is the literal translation of C's
# defined size_t arithmetic.)
run_mutation equivalent "bound: saturating instead of wrapping add" \
  'buffer.offset.wrapping_add(index) < buffer.length' \
  'buffer.offset.saturating_add(index) < buffer.length'

echo
echo "### NULL guards (ERRORS.md E1/E2/E10)"
run_mutation kill "guard: drop the content == NULL check" \
  'if input_buffer.is_null() || (*input_buffer).content.is_null() {' \
  'if input_buffer.is_null() {'
run_mutation kill "guard: add an item == NULL check the C does not have" \
  'if input_buffer.is_null() || (*input_buffer).content.is_null() {' \
  'if item.is_null() || input_buffer.is_null() || (*input_buffer).content.is_null() {'

echo
echo "### strtod result handling (ERRORS.md E4)"
run_mutation kill "E4: invert the zero-consumption test" \
  'if start == after_end {' 'if start != after_end {'
run_mutation kill "E4: return true instead of false" \
  'return CJSON_FALSE; /* parse_error */' 'return CJSON_TRUE;'
run_mutation kill "memcpy: read from content instead of content+offset" \
  'buffer.content.add(buffer.offset),' 'buffer.content,'
run_mutation kill "NUL terminator: write '9' instead of 0" \
  'number_c_string.push(0);' "number_c_string.push(b'9');"

echo
echo "### output fields and return value"
run_mutation kill "type: cJSON_Number -> 0" \
  '(*item).type_ = cJSON_Number;' '(*item).type_ = 0;'
run_mutation kill "type: cJSON_Number -> 1 << 4" \
  '(*item).type_ = cJSON_Number;' '(*item).type_ = 1 << 4;'
run_mutation kill "return: true -> 2 (non-canonical boolean)" \
  '    drop(number_c_string);\n    CJSON_TRUE' '    drop(number_c_string);\n    2'
run_mutation kill "valueint: truncate -> round" \
  '(*item).valueint = double_to_int_trunc(number);' \
  '(*item).valueint = number.round() as c_int;'
run_mutation kill "valueint: truncate -> floor" \
  '(*item).valueint = double_to_int_trunc(number);' \
  '(*item).valueint = number.floor() as c_int;'
run_mutation kill "valuedouble: negate" \
  '(*item).valuedouble = number;' '(*item).valuedouble = -number;'

echo
echo "### offset advance"
run_mutation kill "offset: advance by run length instead of consumed bytes" \
  'buffer.offset = buffer.offset.wrapping_add(consumed);' \
  'buffer.offset = buffer.offset.wrapping_add(number_string_length);'
run_mutation kill "offset: off-by-one advance" \
  'buffer.offset = buffer.offset.wrapping_add(consumed);' \
  'buffer.offset = buffer.offset.wrapping_add(consumed).wrapping_add(1);'
run_mutation kill "offset: overwrite instead of advance" \
  'buffer.offset = buffer.offset.wrapping_add(consumed);' \
  'buffer.offset = consumed;'
run_mutation kill "offset: also advance on the parse_error path" \
  '        return CJSON_FALSE; /* parse_error */' \
  '        buffer.offset = buffer.offset.wrapping_add(1);\n        return CJSON_FALSE;'
run_mutation kill "order: update offset BEFORE writing item (aliasing-visible)" \
  '    (*item).valuedouble = number;' \
  '    let c0 = (after_end as usize).wrapping_sub(start as usize);\n    buffer.offset = buffer.offset.wrapping_add(c0);\n    (*item).valuedouble = number;' \
  '    buffer.offset = buffer.offset.wrapping_add(consumed);' \
  '    let _ = consumed;'
# EQUIVALENT: `type` (offset 0) and `valueint` (offset 4) are distinct
# addresses, and nothing between the two stores reads either field or any
# location they could alias (`buffer.offset` is read only afterwards, in both
# variants). Reordering two independent stores is unobservable.
run_mutation equivalent "order: write type BEFORE valueint" \
  '    (*item).type_ = cJSON_Number;\n\n    let consumed' \
  '    let consumed' \
  '    /* use saturation in case of overflow */' \
  '    (*item).type_ = cJSON_Number;\n    /* use saturation in case of overflow */'

echo
echo "=========================================================="
echo "caught:                       $caught"
echo "expected (equivalent) survivors: $expected_survivors"
echo "unexpected survivors / errors:   $unexpected"
if [ "$unexpected" -eq 0 ]; then
  echo "RESULT: no blind spots detected"
else
  echo "RESULT: the suite has blind spots"
fi
exit "$unexpected"
