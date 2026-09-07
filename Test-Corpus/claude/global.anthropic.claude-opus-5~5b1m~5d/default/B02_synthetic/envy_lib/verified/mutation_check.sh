#!/usr/bin/env bash
# Mutation self-check for the differential suite.
#
# Deliberately breaks the Rust translation in one place at a time, re-runs the
# full differential suite, and reports whether the suite noticed. This is what
# proves the tests have teeth: a suite that passes against a stale or a subtly
# wrong .so is worthless.
#
# Every mutation below MUST be reported DETECTED, with one documented exception:
#
#   "colon pos %ld -> %d"  is an EQUIVALENT mutant, not a miss. `envy` prints
#   `colon_pos - buffer`, which is always exactly 6. On the x86-64 SysV ABI the
#   argument travels in a full 64-bit register either way, so `%d` reads the low
#   32 bits of the same register and printf emits the identical byte "6". There
#   is no input that can distinguish the two format strings, so no test can
#   (or should) fail.
#
# Usage:  cd translation && bash mutation_check.sh
set -u

backup="$(mktemp)"
patcher="$(mktemp /tmp/patch_XXXX.py 2>/dev/null || mktemp)"
trap 'cp "$backup" src/lib.rs; rm -f "$backup" "$patcher"' EXIT
cp src/lib.rs "$backup"
ORIG="$backup"

cat > "$patcher" <<'PYEOF'
import sys
path = "src/lib.rs"
s = open(path).read()
old, new = sys.argv[1], sys.argv[2]
if s.count(old) < 1:
    sys.stderr.write("PATTERN NOT FOUND: %r\n" % old)
    sys.exit(2)
open(path, "w").write(s.replace(old, new, 1))
PYEOF

run_mut () {
  local name="$1"; shift
  cp "$ORIG" src/lib.rs
  if ! python3 "$patcher" "$1" "$2" 2>/dev/null; then
    echo "MUT $name: PATTERN-NOT-FOUND"
    cp "$ORIG" src/lib.rs
    return
  fi
  out=$(timeout 600 cargo test --release 2>&1)
  if printf '%s' "$out" | grep -q 'test result: FAILED'; then
    n=$(printf '%s' "$out" | grep -c '^test .* FAILED')
    echo "MUT $name: DETECTED ($n failing tests)"
  elif printf '%s' "$out" | grep -q '^error'; then
    echo "MUT $name: BUILD-ERROR (mutation invalid)"
  else
    echo "MUT $name: *** NOT DETECTED ***"
  fi
}

run_mut "log_level 3->4"          'cf_set_log_level(flags, 0o3)' 'cf_set_log_level(flags, 0o4)'
run_mut "cache OR 0x0F->0x1F"     'adjusted |= 0x0F;' 'adjusted |= 0x1F;'
run_mut "param4 >>2 -> >>3"       'result.wrapping_add(param4 >> 2)' 'result.wrapping_add(param4 >> 3)'
run_mut "verbose <<1 -> <<2"      '((adjusted as u32) << 1)' '((adjusted as u32) << 2)'
run_mut "semicolon -> colon"      "strchr(env_value, b';' as c_int)" "strchr(env_value, b':' as c_int)"
run_mut "base default 0100->0101" 'cstr(S_PROG_BASE_OFFSET), 0o100' 'cstr(S_PROG_BASE_OFFSET), 0o101'
run_mut "mult default 012->013"   'cstr(S_PROG_MULTIPLIER), 0o12' 'cstr(S_PROG_MULTIPLIER), 0o13'
run_mut "val2/2 -> val2/3"        'val2.wrapping_div(2)' 'val2.wrapping_div(3)'
run_mut "verbose drops strchr"    "!verbose_env.is_null() && !strchr(verbose_env, b'1' as c_int).is_null()" '!verbose_env.is_null()'
run_mut "optimize requires '1'"   'cf_set_optimize(flags, if !optimize_env.is_null() { 1 } else { 0 });' "cf_set_optimize(flags, if !optimize_env.is_null() && !strchr(optimize_env, b'1' as c_int).is_null() { 1 } else { 0 });"
run_mut "rollback <0 -> <=0"      'if result < 0 {' 'if result <= 0 {'
run_mut "bitfield whole-word store" 'base.write((byte & !(mask << shift)) | (((value as u8) & mask) << shift));' '(p as *mut [u8;4]).write([(byte & !(mask << shift)) | (((value as u8) & mask) << shift), 0, 0, 0]);'
run_mut "comma warning text"      'b"Warning: Invalid character in %s' 'b"Warning: invalid character in %s'
run_mut "octal %o -> %d"          'operation_mode = %o (octal)' 'operation_mode = %d (octal)'
run_mut "colon pos %ld -> %d"     'Found colon at position: %ld' 'Found colon at position: %d'
run_mut "rollback value -> 0"     'result = (*state).base_value;' 'result = 0;'
run_mut "DEBUG_SHIFT 1->2"        'const DEBUG_SHIFT: u32 = 1;' 'const DEBUG_SHIFT: u32 = 2;'
run_mut "log_level width 3->2"    'cf_get(p, LOG_LEVEL_SHIFT, 3)' 'cf_get(p, LOG_LEVEL_SHIFT, 2)'
run_mut "hand-rolled atoi"        'atoi(env_value)' '{ let mut n = 0i32; let mut p = env_value; while *p >= 48 && *p <= 57 { n = n.wrapping_mul(10).wrapping_add((*p as i32) - 48); p = p.add(1); } n }'
run_mut "param3 !=0 -> >0"        'if param3 != 0 {' 'if param3 > 0 {'
run_mut "param4 !=0 -> >0"        'if param4 != 0 {' 'if param4 > 0 {'
run_mut "reserved not cleared"    'cf_set_reserved(flags, 0);' ''
run_mut "cache_enabled 1->0"      'cf_set_cache_enabled(flags, 1);' 'cf_set_cache_enabled(flags, 0);'
run_mut "base_offset added twice" 'result = result.wrapping_add(base_offset);' 'result = result.wrapping_add(base_offset).wrapping_add(base_offset);'
run_mut "optimize add -> sub"     'result = val1.wrapping_add(val2);' 'result = val1.wrapping_sub(val2);'
run_mut "comma check removed"     "let mut invalid_char: *mut c_char = strchr(env_value, b',' as c_int);" 'let mut invalid_char: *mut c_char = core::ptr::null_mut();'
run_mut "printf order swapped"    'printf(cstr(S_DBG_OPERATION_MODE), operation_mode);
        printf(cstr(S_DBG_RESULT_BEFORE), result);' 'printf(cstr(S_DBG_RESULT_BEFORE), result);
        printf(cstr(S_DBG_OPERATION_MODE), operation_mode);'
run_mut "warning to stdout"       'fprintf(stderr, cstr(S_WARN_INVALID_CHAR), env_name);' 'printf(cstr(S_WARN_INVALID_CHAR), env_name);'

cp "$ORIG" src/lib.rs
echo
echo "restored original src/lib.rs"
echo "NOTE: 'colon pos %ld -> %d' is an equivalent mutant; see the header comment."
