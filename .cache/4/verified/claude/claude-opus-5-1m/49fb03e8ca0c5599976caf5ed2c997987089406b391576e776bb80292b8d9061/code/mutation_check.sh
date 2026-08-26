#!/usr/bin/env bash
# Harness validation: every mutant of src/lib.rs below MUST be rejected by the
# differential test suite.  If a mutant survives, the tests are not actually
# comparing what they claim to compare.
#
# Usage: ./mutation_check.sh
set -uo pipefail
cd "$(dirname "$0")"

WORK="${TMPDIR:-/tmp}/mutants"
mkdir -p "$WORK"

# name|sed expression
MUTANTS=(
  'log_level_3_to_4|s/flags.set_log_level(0o3)/flags.set_log_level(4)/'
  'base_offset_default|s/parse_env_numeric(ENV_PROG_BASE_OFFSET.as_ptr() as \*const c_char, 0o100)/parse_env_numeric(ENV_PROG_BASE_OFFSET.as_ptr() as *const c_char, 100)/'
  'multiplier_default|s/ENV_PROG_MULTIPLIER.as_ptr() as \*const c_char, 0o12/ENV_PROG_MULTIPLIER.as_ptr() as *const c_char, 12/'
  'verbose_presence_only|s/if !verbose_env.is_null() \&\& !strchr(verbose_env, b.1. as c_int).is_null()/if !verbose_env.is_null()/'
  'cache_mask_0e|s/adjusted |= 0x0F;/adjusted |= 0x0E;/'
  'param4_logical_shift|s/result.wrapping_add(param4 >> 2)/result.wrapping_add(((param4 as u32) >> 2) as c_int)/'
  'val2_div_to_shift|s/val2.wrapping_div(2)/val2 >> 1/'
  'skip_comma_check|s/strchr(env_value, b.,. as c_int)/strchr(env_value, b\x27!\x27 as c_int)/'
  'flags_store_whole_word|s/flags_store(flags_out, tmp);/core::ptr::write_volatile(flags_out.cast::<c_uint>(), tmp.bits \& 0xFF);/'
  'result_le_zero|s/if result < 0 {/if result <= 0 {/'
  'verbose_banner_text|s/b"Verbose mode enabled/b"verbose mode enabled/'
  'reserved_bit_one|s/flags.set_reserved(0)/flags.set_reserved(1)/'
  'result_string_shorter|s/b"Result:%d:Complete/b"Res:%d:Complete/'
  'optimize_branch_swap|s/result = val1.wrapping_add(val2);/result = val1.wrapping_sub(val2);/'
  'semicolon_warning_text|s/b"Warning: Semicolon found in %s/b"Warning: semicolon found in %s/'
  'apply_shift_by_two|s/((adjusted as u32) << 1) as c_int/((adjusted as u32) << 2) as c_int/'
  # these two must be caught by the NULL / misaligned pointer rows of ERRORS.md
  'flags_load_via_reference|s/let flags = \&flags_load(flags);/let flags = \&*flags;/'
  'flags_load_4byte_aligned|s/bits: c_uint::from_ne_bytes(bytes),/bits: core::ptr::read_volatile(p.cast::<c_uint>()),/'
  'flags_load_wrong_offset|s/read_volatile(b.wrapping_add(1))/read_volatile(b.wrapping_add(2))/'
)

pass=0
fail=0
for entry in "${MUTANTS[@]}"; do
  name="${entry%%|*}"
  expr="${entry#*|}"
  src="$WORK/$name.rs"
  so="$WORK/lib${name}.so"
  sed "$expr" src/lib.rs > "$src"
  if cmp -s "$src" src/lib.rs; then
    echo "SKIP  $name (sed produced no change - fix the pattern)"
    fail=$((fail+1))
    continue
  fi
  if ! rustc --crate-name envy_lib --crate-type cdylib --edition 2021 \
        -C debug-assertions=on -C overflow-checks=on "$src" -o "$so" \
        >"$WORK/$name.build.log" 2>&1; then
    echo "SKIP  $name (mutant does not compile; see $WORK/$name.build.log)"
    fail=$((fail+1))
    continue
  fi
  if ENVY_RUST_SO="$so" timeout 600 cargo test --offline -q >"$WORK/$name.test.log" 2>&1; then
    echo "SURVIVED  $name  <-- the test suite failed to detect this mutation"
    fail=$((fail+1))
  else
    killed=$(grep -c "MISMATCH" "$WORK/$name.test.log")
    echo "killed    $name  ($killed mismatching comparisons reported)"
    pass=$((pass+1))
  fi
done

echo
echo "mutants killed: $pass, problems: $fail"
[ "$fail" -eq 0 ]
