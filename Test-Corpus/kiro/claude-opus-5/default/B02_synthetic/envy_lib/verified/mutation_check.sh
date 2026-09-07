#!/usr/bin/env bash
# Harness self-check: deliberately mutate the Rust translation, confirm the
# differential test suite FAILS, then restore. A mutation that survives means
# the suite has a blind spot.
set -u
cd "$(dirname "$0")"

SRC=src/lib.rs
cp "$SRC" /tmp/lib.rs.orig
trap 'cp /tmp/lib.rs.orig "$SRC"' EXIT

declare -a NAMES=()
declare -a FROM=()
declare -a TO=()

add() { NAMES+=("$1"); FROM+=("$2"); TO+=("$3"); }

add "base_offset default 0o100 -> 0o101"        '0o100'                     '0o101'
add "multiplier default 0o12 -> 0o13"           'c"PROG_MULTIPLIER".as_ptr(), 0o12' 'c"PROG_MULTIPLIER".as_ptr(), 0o13'
add "log_level 0o3 -> 0o4"                      'log_level: 0o3'            'log_level: 0o4'
add "cache_enabled 1 -> 0"                      'cache_enabled: 1,'         'cache_enabled: 0,'
add "OR mask 0x0F -> 0x1F"                      'adjusted |= 0x0F'          'adjusted |= 0x1F'
add "verbose shift 1 -> 2"                      '(adjusted as u32) << 1'    '(adjusted as u32) << 2'
add "param4 >> 2 -> >> 3"                       'param4 >> 2'               'param4 >> 3'
add "val2/2 -> val2/3"                          'wrapping_div(2)'           'wrapping_div(3)'
add "recovery test < 0 -> <= 0"                 'if result < 0 {'           'if result <= 0 {'
add "comma check ,  -> ."                       "strchr(env_value, b',' as c_int)" "strchr(env_value, b'.' as c_int)"
add "semicolon check ; -> :"                    "strchr(env_value, b';' as c_int)" "strchr(env_value, b':' as c_int)"
add "verbose probe '1' -> '2'"                  "strchr(verbose_env, b'1' as c_int)" "strchr(verbose_env, b'2' as c_int)"
add "debug probe '1' -> '2'"                    "strchr(debug_env, b'1' as c_int)"   "strchr(debug_env, b'2' as c_int)"
add "optimize null test inverted"               'let optimize = if !optimize_env.is_null()' 'let optimize = if optimize_env.is_null()'
add "operation_mode 0o755 -> 0o756"             '0o755'                     '0o756'
add "warning text Invalid -> invalid"           'Warning: Invalid character' 'Warning: invalid character'
add "warning text Semicolon -> Semi-colon"      'Warning: Semicolon found'  'Warning: Semi-colon found'
add "verbose banner text"                       'Verbose mode enabled\n'    'Verbose mode Enabled\n'
add "colon offset message %ld -> %d"            'Found colon at position: %ld\n' 'Found colon at position: %d\n'
add "log_level mask 0x07 -> 0x0F"               'LOG_LEVEL_MASK: u8 = 0x07' 'LOG_LEVEL_MASK: u8 = 0x0F'
add "log_level shift 4 -> 5"                    'LOG_LEVEL_SHIFT: u32 = 4'  'LOG_LEVEL_SHIFT: u32 = 5'
add "reserved shift 7 -> 6"                     'RESERVED_SHIFT: u32 = 7'   'RESERVED_SHIFT: u32 = 6'
add "recovery returns param2 not base_value"    'result = state.base_value;' 'result = param2;'
add "param3 term uses param4"                   'param3.wrapping_mul(state.multiplier)' 'param4.wrapping_mul(state.multiplier)'
add "drop the envy export"                      '#[unsafe(no_mangle)]
pub unsafe extern "C" fn envy(' 'pub unsafe extern "C" fn envy('
add "flags read from wrong byte"                'flags as *const u8' '(flags as *const u8).add(1)'

survived=0
for i in "${!NAMES[@]}"; do
  cp /tmp/lib.rs.orig "$SRC"
  python3 - "$SRC" "${FROM[$i]}" "${TO[$i]}" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
frm = frm.replace('\\n', '\n'); to = to.replace('\\n', '\n')
s = open(path).read()
if frm not in s:
    print("PATTERN-NOT-FOUND"); sys.exit(3)
open(path, 'w').write(s.replace(frm, to, 1))
PY
  rc=$?
  if [ $rc -eq 3 ]; then
    printf 'SKIP (pattern absent) : %s\n' "${NAMES[$i]}"
    survived=$((survived+1))
    continue
  fi

  if ! cargo build --release >/tmp/mut_build.log 2>&1; then
    printf 'SKIP (does not compile): %s\n' "${NAMES[$i]}"
    survived=$((survived+1))
    continue
  fi

  out=$(timeout 900 cargo test --release -- --test-threads=1 2>&1)
  if printf '%s' "$out" | grep -qE '^test result: FAILED|panicked at'; then
    first=$(printf '%s' "$out" | grep -oE '^test [a-z0-9_]+ \.\.\. FAILED' | head -3 | sed 's/^test //;s/ \.\.\. FAILED//' | paste -sd, -)
    printf 'CAUGHT  : %-45s  by: %s\n' "${NAMES[$i]}" "${first:-symbol/compile check}"
  else
    printf 'SURVIVED: %-45s  <-- BLIND SPOT\n' "${NAMES[$i]}"
    survived=$((survived+1))
  fi
done

cp /tmp/lib.rs.orig "$SRC"
cargo build --release >/dev/null 2>&1
echo "----"
echo "mutations: ${#NAMES[@]}, survived: $survived"
exit $((survived > 0))
