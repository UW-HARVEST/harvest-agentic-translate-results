#!/usr/bin/env bash
# Mutation testing: proves the differential suite is actually SENSITIVE rather
# than vacuously passing.
#
# For each mutation we plant a deliberate bug in src/lib.rs, rebuild the cdylib,
# run the suite, and record whether the suite caught it. src/lib.rs is always
# restored afterwards (also on Ctrl-C / error).
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGDIR="${TMPDIR:-/tmp}/mutate-logs"
mkdir -p "$LOGDIR"
SRC="$here/src/lib.rs"
BAK="$LOGDIR/lib.rs.orig"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

caught=0; missed=0
declare -a MISSED_LIST=()

# run_mutation <name> <expect: CAUGHT|UNOBSERVABLE> <python-replacement-code>
run_mutation() {
  local name="$1" expect="$2" code="$3"
  restore
  python3 - "$SRC" <<PY
import sys
p = sys.argv[1]
s = open(p).read()
before = s
$code
assert s != before, "mutation '$name' did not change the source"
open(p, 'w').write(s)
PY
  if [[ $? -ne 0 ]]; then
    printf '\033[33mSKIP\033[0m %-52s (pattern not found)\n' "$name"
    return
  fi

  if ! cargo build --offline >"$LOGDIR/build.log" 2>&1; then
    printf '\033[33mSKIP\033[0m %-52s (mutant does not compile)\n' "$name"
    return
  fi

  if RUST_DRIVER_SO="$here/target/debug/libdriver.so" timeout 600 \
       cargo test --offline -- --test-threads=4 >"$LOGDIR/test.log" 2>&1; then
    # suite passed -> mutation NOT caught
    if [[ $expect == UNOBSERVABLE ]]; then
      printf '\033[36m----\033[0m %-52s not caught (expected: provably unobservable)\n' "$name"
    else
      printf '\033[31mMISS\033[0m %-52s BUG WENT UNDETECTED\n' "$name"
      missed=$((missed+1)); MISSED_LIST+=("$name")
    fi
  else
    local n
    n=$(grep -c '^test .* FAILED$' "$LOGDIR/test.log")
    printf '\033[32mCAUGHT\033[0m %-50s by %s failing test(s)\n' "$name" "$n"
    caught=$((caught+1))
  fi
}

printf '\n\033[1m== Mutation testing the differential suite ==\033[0m\n\n'

run_mutation "drop 'E' from the accepted byte set" CAUGHT \
  "s = s.replace(\"| b'e' | b'E' =>\", \"| b'e' =>\")"

run_mutation "drop '+' from the accepted byte set" CAUGHT \
  "s = s.replace(\"b'9' | b'+' | b'-'\", \"b'9' | b'-'\")"

run_mutation "drop '.' from the accepted byte set" CAUGHT \
  "s = s.replace(\"b'.' => {\", \"b'\\\\x01' => {\")"

run_mutation "accept 'x' as well (hex-float leak)" CAUGHT \
  "s = s.replace(\"| b'e' | b'E' =>\", \"| b'e' | b'E' | b'x' | b'X' =>\")"

run_mutation "accept ' ' as well (whitespace leak)" CAUGHT \
  "s = s.replace(\"| b'e' | b'E' =>\", \"| b'e' | b'E' | b' ' =>\")"

run_mutation "offset += scan length instead of after_end-start" CAUGHT \
  "s = s.replace('.wrapping_add((after_end as usize).wrapping_sub(number_c_string as usize));', '.wrapping_add(number_string_length);')"

run_mutation "remove the content==NULL check" CAUGHT \
  "s = s.replace('if input_buffer.is_null() || unsafe { (*input_buffer).content }.is_null() {', 'if input_buffer.is_null() {')"

run_mutation "cJSON_Number = 1<<4 instead of 1<<3" CAUGHT \
  "s = s.replace('const cJSON_Number: c_int = 1 << 3;', 'const cJSON_Number: c_int = 1 << 4;')"

run_mutation "return true on the strtod-consumed-nothing path" CAUGHT \
  "s = s.replace('return CJSON_FALSE; /* parse_error */', 'return CJSON_TRUE; /* parse_error */')"

run_mutation "off-by-one: scan bound uses <= length" CAUGHT \
  "s = s.replace('(*buffer).offset.wrapping_add(index) < (*buffer).length', '(*buffer).offset.wrapping_add(index) <= (*buffer).length')"

run_mutation "malloc one byte too small (no room for NUL)" CAUGHT \
  "s = s.replace('malloc(number_string_length.wrapping_add(1))', 'malloc(number_string_length)')"

run_mutation "INT_MIN saturation bound off by one" CAUGHT \
  "s = s.replace('} else if number <= INT_MIN as c_double {', '} else if number <= (INT_MIN + 1) as c_double {')"

run_mutation "INT_MAX saturation bound off by one" CAUGHT \
  "s = s.replace('if number >= INT_MAX as c_double {', 'if number >= (INT_MAX - 1) as c_double {')"

run_mutation "truncate away from zero (floor) instead of toward zero" CAUGHT \
  "s = s.replace('number as c_int', 'number.floor() as c_int')"

run_mutation "write valuedouble only, never valueint" CAUGHT \
  "s = s.replace('item_store!(item, valueint, INT_MAX);', '{}')"

run_mutation "never write the type field" CAUGHT \
  "s = s.replace('item_store!(item, type_, cJSON_Number);', '{}')"

run_mutation "write valuedouble to the valueint slot" CAUGHT \
  "s = s.replace('item_store!(item, valuedouble, number);', 'item_store!(item, valueint, number as c_int);')"

run_mutation "skip the NUL terminator" CAUGHT \
  "s = s.replace(\"*number_c_string.wrapping_add(number_string_length) = b'\\\\0';\", '()')"

# Regression guard for the one real divergence this verification found: a plain
# place-expression store makes rustc emit a null-pointer-dereference UB check
# under -C debug-assertions, so a NULL \`item\` aborts (SIGABRT + a Rust panic
# message) where the C raises SIGSEGV.
run_mutation "place-expr store reintroduces the null-deref check" CAUGHT \
  "s = s.replace('item_store!(item, valuedouble, number);', 'unsafe { (*item).valuedouble = number };')"

# --- mutations that are PROVABLY unobservable -------------------------------
# Kept in the suite as documentation: they change the source but cannot change
# behaviour, so "not caught" is the correct outcome, not a gap in the tests.
run_mutation "'>=' -> '>' at the INT_MAX bound" UNOBSERVABLE \
  "s = s.replace('if number >= INT_MAX as c_double {', 'if number > INT_MAX as c_double {')"

run_mutation "'<=' -> '<' at the INT_MIN bound" UNOBSERVABLE \
  "s = s.replace('} else if number <= INT_MIN as c_double {', '} else if number < INT_MIN as c_double {')"

run_mutation "never set has_decimal_point (loop is a no-op)" UNOBSERVABLE \
  "s = s.replace('has_decimal_point = CJSON_TRUE;', 'has_decimal_point = CJSON_FALSE;')"

restore
printf '\n\033[1m== RESULT ==\033[0m\n'
printf 'caught: %d   missed: %d\n' "$caught" "$missed"
if (( missed > 0 )); then
  printf '\033[31mundetected mutations:\033[0m\n'; printf '  %s\n' "${MISSED_LIST[@]}"
  exit 1
fi
printf '\033[32mevery behaviour-changing mutation was detected\033[0m\n'
