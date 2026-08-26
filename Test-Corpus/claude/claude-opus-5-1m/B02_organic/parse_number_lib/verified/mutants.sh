#!/usr/bin/env bash
# Harness self-check: deliberately inject bugs into src/lib.rs and confirm the
# differential suite CATCHES each one. A "SURVIVED" mutant means the test suite
# has a blind spot (unless the mutant is provably semantics-preserving).
#
# Detection is measured by the test-runner EXIT CODE, so a mutant that makes the
# Rust crash (SIGSEGV/SIGABRT) counts as caught even though libtest prints no
# "FAILED" line in that case.
set -uo pipefail
cd "$(dirname "$0")"

BAK="${TMPDIR:-.}/lib.rs.mutants.bak"
LOG="${TMPDIR:-.}/mutants.log"
cp src/lib.rs "$BAK"
trap 'cp "$BAK" src/lib.rs' EXIT

KILLED=0; SURVIVED=0; SURVIVORS=()

mutate() {
  local desc="$1" old="$2" new="$3"
  cp "$BAK" src/lib.rs
  if ! python3 -c "
import sys, pathlib
p = pathlib.Path('src/lib.rs'); s = p.read_text()
if sys.argv[1] not in s: sys.exit(3)
p.write_text(s.replace(sys.argv[1], sys.argv[2], 1))
" "$old" "$new"; then
    printf '%-16s %s\n' "SKIP(pattern)" "$desc"; return
  fi
  timeout 600 cargo test --offline >"$LOG" 2>&1
  local rc=$?
  if [ "$rc" -ne 0 ]; then
    KILLED=$((KILLED + 1))
    printf '\033[32m%-16s\033[0m %-44s (rc=%s, %s failing test(s))\n' \
      KILLED "$desc" "$rc" "$(grep -cE '^test .* FAILED' "$LOG")"
  else
    SURVIVED=$((SURVIVED + 1)); SURVIVORS+=("$desc")
    printf '\033[31m%-16s\033[0m %-44s (rc=0)\n' SURVIVED "$desc"
  fi
}

echo "=== Mutation testing the differential suite ==="

# --- charset / scanning loop ------------------------------------------------
mutate "charset drops 'e'"              "| b'e'" "| b'q'"
mutate "charset drops '+'"              "| b'+'" "| b'\\x02'"
mutate "charset drops '.'"              "b'.' => {" "b'\\x01' => {"
mutate "charset gains ' '"              "| b'e'" "| b' ' | b'e'"
mutate "scan continues past default"    "_ => break, /* goto loop_end */" "_ => {}, /* goto loop_end */"

# --- can_access_at_index ---------------------------------------------------
mutate "can_access <= not <"            'offset.wrapping_add(index) < length' 'offset.wrapping_add(index) <= length'
mutate "can_access off-by-one low"      'offset.wrapping_add(index) < length' 'offset.wrapping_add(index) + 1 < length'
mutate "can_access ignores offset"      'offset.wrapping_add(index) < length' 'index < length'
mutate "can_access saturating add"      'offset.wrapping_add(index) < length' 'offset.saturating_add(index) < length'

# --- NULL checks -----------------------------------------------------------
mutate "no content NULL check"          'if input_buffer.is_null() || unsafe { pb_content(input_buffer).is_null() } {' 'if input_buffer.is_null() {'
mutate "no buffer NULL check"           'if input_buffer.is_null() || unsafe { pb_content(input_buffer).is_null() } {' 'if false {'

# --- temporary buffer / strtod --------------------------------------------
mutate "no NUL terminator"              "number_c_string.push(b'\\0');" "number_c_string.push(b'9');"
mutate "copies one byte too few"        'number_string_length,
            );' 'number_string_length.saturating_sub(1),
            );'
mutate "strtod failure ignored"         'if start == after_end {' 'if false {'
mutate "strtod failure always"          'if start == after_end {' 'if true {'
mutate "returns true on parse_error"    'return CJSON_FALSE; /* parse_error */' 'return CJSON_TRUE; /* parse_error */'
mutate "returns false on success"       '    CJSON_TRUE
}' '    CJSON_FALSE
}'

# --- saturation / int conversion -----------------------------------------
mutate "sat INT_MAX -> INT_MIN"         'store_valueint(item, C_INT_MAX);' 'store_valueint(item, C_INT_MIN);'
mutate "sat branches swapped"           'store_valueint(item, C_INT_MAX);
        } else if number <= C_INT_MIN as f64 {
            /* item->valueint = INT_MIN; */
            store_valueint(item, C_INT_MIN);' 'store_valueint(item, C_INT_MIN);
        } else if number <= C_INT_MIN as f64 {
            /* item->valueint = INT_MIN; */
            store_valueint(item, C_INT_MAX);'
mutate "int cast rounds"                'store_valueint(item, double_to_int_c(number));' 'store_valueint(item, double_to_int_c(number.round()));'
mutate "int cast ceils"                 'store_valueint(item, double_to_int_c(number));' 'store_valueint(item, double_to_int_c(number.ceil()));'
mutate "NaN maps to 0 not INT_MIN"      'if number.is_nan() {
        return C_INT_MIN;' 'if number.is_nan() {
        return 0;'

# --- out-parameter writes -------------------------------------------------
mutate "cJSON_Number constant wrong"    'const CJSON_NUMBER: c_int = 1 << 3;' 'const CJSON_NUMBER: c_int = 1 << 4;'
mutate "type not written"               'store_type(item, CJSON_NUMBER);' '{}'
mutate "valuedouble not written"        'store_double(item, number);' '{}'
mutate "item written on failure"        'drop(number_c_string);
        return CJSON_FALSE; /* parse_error */' 'store_type(item, CJSON_NUMBER);
        drop(number_c_string);
        return CJSON_FALSE; /* parse_error */'
mutate "depth clobbered"                'store_type(item, CJSON_NUMBER);' 'store_type(item, CJSON_NUMBER);
        core::ptr::write_unaligned(&raw mut (*input_buffer).depth, 0usize);'
mutate "length clobbered"               'store_type(item, CJSON_NUMBER);' 'store_type(item, CJSON_NUMBER);
        core::ptr::write_unaligned(&raw mut (*input_buffer).length, 0usize);'

# --- offset arithmetic ---------------------------------------------------
mutate "offset += full scanned run"     'pb_offset(input_buffer).wrapping_add(after_end as usize - start as usize)' 'pb_offset(input_buffer).wrapping_add(number_string_length)'
mutate "offset overwritten not added"   'pb_offset(input_buffer).wrapping_add(after_end as usize - start as usize)' '(after_end as usize - start as usize)'
mutate "offset not advanced"            'pb_set_offset(' 'let _unused = ('

# --- decimal-point rewrite ----------------------------------------------
mutate "decimal_point becomes ','"      "let decimal_point: c_uchar = b'.';" "let decimal_point: c_uchar = b',';"
mutate "rewrite loop always runs"       'if has_decimal_point != CJSON_FALSE {' 'if true {'

# --- debug-assertion / UB-check regressions (the real bugs found) --------
mutate "item store: checked deref"      'unsafe { core::ptr::write_unaligned(&raw mut (*item).valuedouble, v) }' 'unsafe { (*item).valuedouble = v }'
mutate "buffer read: checked deref"     'unsafe { core::ptr::read_unaligned(&raw const (*buffer).length) }' 'unsafe { (*buffer).length }'

echo
echo "=== killed: $KILLED   survived: $SURVIVED ==="
if [ "$SURVIVED" -gt 0 ]; then
  echo "survivors (must each be justified as a semantics-preserving mutant):"
  printf '  - %s\n' "${SURVIVORS[@]}"
fi
