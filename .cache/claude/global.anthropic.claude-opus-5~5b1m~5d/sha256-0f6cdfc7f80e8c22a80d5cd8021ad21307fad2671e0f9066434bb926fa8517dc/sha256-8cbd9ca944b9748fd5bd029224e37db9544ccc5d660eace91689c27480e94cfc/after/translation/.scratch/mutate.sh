#!/usr/bin/env bash
# Negative control: inject a known bug into the Rust lib, rebuild, and confirm
# the differential suite CATCHES it. Restores the pristine lib.rs at the end.
set -u
cd "$(dirname "$0")/.."
ORIG=.scratch/lib.rs.orig

run_case() {
  local name="$1" old="$2" new="$3"
  cp "$ORIG" src/lib.rs
  OLD="$old" NEW="$new" python3 -c '
import os
p="src/lib.rs"
s=open(p).read()
old=os.environ["OLD"].encode().decode("unicode_escape")
new=os.environ["NEW"].encode().decode("unicode_escape")
assert old in s, "PATTERN NOT FOUND: "+repr(old)
open(p,"w").write(s.replace(old,new,1))
' || { echo "MUTANT $name -> SKIPPED (pattern not found)"; return; }
  if ! cargo build --release --offline -q >/dev/null 2>&1; then
    echo "MUTANT $name -> SKIPPED (does not compile)"
    return
  fi
  local n
  n=$(timeout 600 cargo test --release --offline 2>&1 | grep -c '^test .* FAILED$')
  if [ "$n" -gt 0 ]; then
    echo "MUTANT $name -> CAUGHT by $n test(s)"
  else
    echo "MUTANT $name -> *** NOT CAUGHT *** (suite has a blind spot)"
  fi
}

run_case "octal-0-prefix-dropped"     'Octal: 0{:o}, Decimal: {}'   'Octal: {:o}, Decimal: {}'
run_case "octal-signed-not-unsigned"  'octal_val as c_uint, octal_val' 'octal_val, octal_val'
run_case "lower-clamp-off-by-one"     'if value < lower_threshold'  'if value <= lower_threshold'
run_case "upper-clamp-off-by-one"     'if value > upper_threshold'  'if value >= upper_threshold'
run_case "normalize-clamps-negatives" 'if is_nonzero != 0 && value > 0' 'if is_nonzero != 0'
run_case "accumulator-guard-boundary" 'if ACCUMULATOR > 0o150'      'if ACCUMULATOR >= 0o150'
run_case "multiplier-guard-boundary"  'if MULTIPLIER > 0o100'       'if MULTIPLIER >= 0o100'
run_case "active-params-add-guard"    'active_params >= mode_add'   'active_params > mode_add'
run_case "active-params-mul-guard"    'active_params >= mode_multiply' 'active_params > mode_multiply'
run_case "needle-not-narrowed"        'let needle = search_char as u8;' 'let needle = search_char as u8; if search_char > 255 { return; }'
run_case "div-guard-changed"          'if b != 0 {'                 'if b != 0 && b != 1 {'
run_case "sentinel-value-changed"     'result = 0o777;'             'result = 0o776;'
run_case "opcount-scale-changed"      'OPERATION_COUNT.wrapping_mul(0o10)' 'OPERATION_COUNT.wrapping_mul(0o11)'
run_case "memchr-p-to-P"              "b'p' as c_int"               "b'P' as c_int"
run_case "div-floors-instead-of-trunc" 'MULTIPLIER.wrapping_div(b)' 'MULTIPLIER.div_euclid(b)'
run_case "replace-replaces-all"       "b'X' as c_char;\\n                return;" "b'X' as c_char;"
run_case "octal-comma-space"          'Octal: 0{:o}, Decimal: {}'   'Octal: 0{:o},Decimal: {}'
run_case "strcpy-no-nul"              '*dest.add(src.len()) = 0;'   '*dest.add(src.len()) = 0; *dest.add(src.len()+1) = 1;'
run_case "findrep-memchr-offset-off"  'result.wrapping_add(offset as c_int)' 'result.wrapping_add(offset as c_int + 1)'
run_case "accumulator-init-wrong"     'static mut ACCUMULATOR: c_int = 0;' 'static mut ACCUMULATOR: c_int = 1;'
run_case "opcount-init-wrong"         'static mut OPERATION_COUNT: c_int = 0;' 'static mut OPERATION_COUNT: c_int = 1;'
run_case "div-no-opcount"             'if b != 0 {\n            MULTIPLIER = MULTIPLIER.wrapping_div(b);\n        }' 'if b != 0 {\n            MULTIPLIER = MULTIPLIER.wrapping_div(b);\n        } else { return MULTIPLIER; }'
run_case "op-table-order-swapped"     'add_to_accumulator,\n    multiply_with_multiplier,' 'multiply_with_multiplier,\n    add_to_accumulator,'
run_case "replace-byte-changed"       "b'X' as c_char"              "b'Y' as c_char"
run_case "multiplier-init-wrong"      'static mut MULTIPLIER: c_int = 1;' 'static mut MULTIPLIER: c_int = 2;'
run_case "sub-operand-order"          'ACCUMULATOR.wrapping_sub(a.wrapping_sub(b))' 'ACCUMULATOR.wrapping_sub(b.wrapping_sub(a))'
run_case "findrep-op2-args-swapped"   'selected_op(normalized_p3, normalized_p4)' 'selected_op(normalized_p4, normalized_p3)'
run_case "findrep-div-divisor"        'selected_op(MULTIPLIER, 2)'  'selected_op(MULTIPLIER, 3)'

cp "$ORIG" src/lib.rs
cargo build --release --offline -q >/dev/null 2>&1
echo "--- restored pristine lib.rs ---"
diff -q src/lib.rs "$ORIG" && echo "lib.rs IDENTICAL to pristine backup"
