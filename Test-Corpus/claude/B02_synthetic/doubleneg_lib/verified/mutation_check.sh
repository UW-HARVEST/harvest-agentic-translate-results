#!/usr/bin/env bash
# Negative control for the differential suite.
#
# Injects known changes into src/lib.rs and checks the suite's verdict:
#   mutate       -> a real behavioural divergence; the suite MUST catch it.
#   mutate_equiv -> provably behaviour-preserving on x86-64; the suite SHOULD
#                   let it pass (catching it would mean the test over-specifies).
#
# A `mutate` that survives is a blind spot in the tests. The script exits
# non-zero if any expectation is violated.
#
# NOTE: `cargo build` is required before every `cargo test` — cargo does not
# regenerate a cdylib-only lib target during `cargo test`.
set -u

ORIG="${TMPDIR:?TMPDIR must be set}/lib.rs.mutbase"
SRC="src/lib.rs"
cp "$SRC" "$ORIG"

bad=0
caught=0
survived=0

revert() { cp "$ORIG" "$SRC"; }
rebuild() { timeout 600 cargo build --no-default-features >/dev/null 2>&1; }

# _run <name> <perl-expr> <test-binary> <expect: catch|survive>
_run() {
  local name="$1" expr="$2" which="$3" expect="$4"
  revert
  perl -0pi -e "$expr" "$SRC"
  if cmp -s "$ORIG" "$SRC"; then
    echo "ERROR    $name -- mutation did not apply (bad regex)"
    bad=$((bad+1)); revert; rebuild; return
  fi

  local verdict
  if ! timeout 600 cargo build --no-default-features >/dev/null 2>&1; then
    verdict=catch   # a compile error is also a detection
  elif timeout 600 cargo test --no-default-features --test "$which" >/dev/null 2>&1; then
    verdict=survive
  else
    verdict=catch
  fi

  if [ "$verdict" = "$expect" ]; then
    if [ "$expect" = catch ]; then
      echo "CAUGHT     $name  ($which)"; caught=$((caught+1))
    else
      echo "SURVIVED*  $name  ($which)  [expected: behaviour-preserving]"; survived=$((survived+1))
    fi
  else
    if [ "$expect" = catch ]; then
      echo "BLIND SPOT $name  ($which)  <-- suite failed to detect a real divergence"
    else
      echo "OVERSPEC   $name  ($which)  <-- suite rejects a behaviour-preserving change"
    fi
    bad=$((bad+1))
  fi
  revert; rebuild
}

mutate()       { _run "$1" "$2" "$3" catch; }
mutate_equiv() { _run "$1" "$2" "$3" survive; }

echo "=== mutations that MUST be caught ==="

# --- convert_double_to_int -------------------------------------------------
mutate "convert: saturate instead of 0x80000000" \
  's/if truncated >= 2147483648\.0 \|\| truncated < -2147483648\.0 \{\n        return i32::MIN;\n    \}/if false { return i32::MIN; }/' phase_c_errors
mutate "convert: NaN -> 0" \
  's/if value\.is_nan\(\) \{\n        return i32::MIN;\n    \}/if value.is_nan() { return 0; }/' phase_c_errors
mutate "convert: floor instead of trunc" \
  's/let truncated = value\.trunc\(\);/let truncated = value.floor();/' phase_b_valid
mutate "convert: round instead of trunc" \
  's/let truncated = value\.trunc\(\);/let truncated = value.round();/' phase_b_valid
mutate "convert: upper bound >= -> > (off by one ULP at 2^31)" \
  's/truncated >= 2147483648\.0/truncated > 2147483648.0/' phase_c_errors

# --- find_value_in_buffer --------------------------------------------------
mutate "find: absent -> 0 instead of -1" \
  's/    -1\n\}/    0\n}/' phase_c_errors
mutate "find: absent -> 1 instead of -1" \
  's/    -1\n\}/    1\n}/' phase_c_errors
mutate "find: null check inverted" \
  's/if !result\.is_null\(\)/if result.is_null()/' phase_c_errors
mutate "find: offset computed backwards" \
  's/\(result as usize\)\.wrapping_sub\(buffer as usize\)/(buffer as usize).wrapping_sub(result as usize)/' phase_c_errors
mutate "find: off-by-one offset" \
  's/as isize as c_int;/as isize as c_int + 1;/' phase_c_errors
mutate "find: size - 1 passed to memchr" \
  's/target as c_int, size\)/target as c_int, size.wrapping_sub(1))/' phase_c_errors

# --- process_negation ------------------------------------------------------
mutate "process_negation: & 1" \
  's/let var2: c_int = if var1 != 0 \{ 1 \} else \{ 0 \};/let var2: c_int = var1 \& 1;/' phase_c_errors
mutate "process_negation: sign test" \
  's/if var1 != 0 \{ 1 \} else \{ 0 \}/if var1 > 0 { 1 } else { 0 }/' phase_c_errors
mutate "process_negation: single negation" \
  's/if var1 != 0 \{ 1 \} else \{ 0 \}/if var1 == 0 { 1 } else { 0 }/' phase_c_errors

# --- create_numeric_buffer -------------------------------------------------
mutate "create: stride 7 -> 6" \
  's/i\.wrapping_mul\(7\)/i.wrapping_mul(6)/' phase_b_valid
mutate "create: stride 7 -> 8" \
  's/i\.wrapping_mul\(7\)/i.wrapping_mul(8)/' phase_b_valid
mutate "create: loop i <= size (writes one extra)" \
  's/while i < size \{/while i <= size {/' phase_b_valid
mutate "create: seed subtracted instead of added" \
  's/seed\.wrapping_add\(i\.wrapping_mul\(7\)\)/seed.wrapping_sub(i.wrapping_mul(7))/' phase_b_valid
mutate "create: stores u8 -> zero-extended (drops signedness)" \
  's/\*buffer\.offset\(i as isize\) = v as i8 as c_char;/*buffer.offset(i as isize) = (v as u8 as i16 \& 0x7f) as c_char;/' phase_c_errors
mutate "create: treats size as unsigned (negative -> huge)" \
  's/while i < size \{/while (i as u32) < (size as u32) {/' phase_c_errors

# --- calculate_with_doubles ------------------------------------------------
mutate "calc: rem_euclid(10) for the exponent" \
  's/c\.wrapping_rem\(10\)/c.rem_euclid(10)/' phase_c_errors
mutate "calc: exponent modulus 10 -> 9" \
  's/c\.wrapping_rem\(10\)/c.wrapping_rem(9)/' phase_c_errors
mutate "calc: b == 0 branch inverted" \
  's/if b != 0 \{\n        result = \(a as f64\) \/ \(b as f64\);/if b == 0 { result = 1.0;/' phase_c_errors
mutate "calc: += instead of *=" \
  's/result \*= unsafe \{ c_pow/result += unsafe { c_pow/' phase_c_errors
mutate "calc: integer division before widening" \
  's/result = \(a as f64\) \/ \(b as f64\);/result = a.wrapping_div(b) as f64;/' phase_b_valid
mutate "calc: pow base 10 -> 2" \
  's/c_pow\(10\.0,/c_pow(2.0,/' phase_c_errors

# --- doubleneg -------------------------------------------------------------
mutate "doubleneg: negation weight 10 -> 1" \
  's/negation_result\.wrapping_mul\(10\)/negation_result.wrapping_mul(1)/' phase_b_doubleneg
mutate "doubleneg: drop neg_p4 from the sum" \
  's/\.wrapping_add\(neg_p3\)\n        \.wrapping_add\(neg_p4\);/.wrapping_add(neg_p3);/' phase_b_doubleneg
mutate "doubleneg: modulo 1000 -> 100" \
  's/\.wrapping_rem\(1000\)/.wrapping_rem(100)/' phase_b_doubleneg
mutate "doubleneg: pow(2,40) -> pow(2,30)" \
  's/c_pow\(2\.0, 40\.0\)/c_pow(2.0, 30.0)/' phase_c_doubleneg_errors
mutate "doubleneg: search_values order swapped" \
  's/param2\.wrapping_rem\(256\),\n        param3\.wrapping_rem\(256\),/param3.wrapping_rem(256),\n        param2.wrapping_rem(256),/' phase_b_doubleneg
mutate "doubleneg: literal 42 -> 43 in search_values" \
  's/        42,\n    \];/        43,\n    ];/' phase_b_doubleneg
mutate "doubleneg: combined loop 10 -> 9 iterations" \
  's/while j < 10 \{/while j < 9 {/' phase_b_doubleneg
mutate "doubleneg: direct memchr byte 100 -> 101" \
  's/bufptr as \*const c_void, 100, 256/bufptr as *const c_void, 101, 256/' phase_b_doubleneg
mutate "doubleneg: buffer seed param1 -> param2" \
  's/create_numeric_buffer\(bufptr, 256, param1\)/create_numeric_buffer(bufptr, 256, param2)/' phase_b_doubleneg
mutate "doubleneg: search_byte i*param2 -> i*param3" \
  's/j\.wrapping_mul\(param2\)/j.wrapping_mul(param3)/' phase_b_doubleneg
mutate "doubleneg: buffer length 256 -> 255 for the searches" \
  's/find_value_in_buffer\(bufptr, 256, sv\)/find_value_in_buffer(bufptr, 255, sv)/' phase_b_doubleneg
mutate "doubleneg: pos >= 0 -> pos > 0 (drops index-0 hits)" \
  's/if pos >= 0 \{/if pos > 0 {/' phase_b_doubleneg

# --- doubleneg, stdout-only divergences -----------------------------------
mutate "doubleneg stdout: %e -> %g" \
  's/Calculated double value: %e/Calculated double value: %g/' phase_b_doubleneg
mutate "doubleneg stdout: %e -> %f" \
  's/Very large negative double: %e/Very large negative double: %f/' phase_b_doubleneg
mutate "doubleneg stdout: section header case changed" \
  's/--- Memchr Search Test ---/--- Memchr Search test ---/' phase_b_doubleneg
mutate "doubleneg stdout: header newline removed" \
  's/cprint!\("\\n--- Combined Feature Test/cprint!("--- Combined Feature Test/' phase_b_doubleneg
mutate "doubleneg stdout: trailing newline removed" \
  's/Accumulated result: %d\\n/Accumulated result: %d/' phase_b_doubleneg
mutate "doubleneg stdout: Parameters order swapped" \
  's/Parameters: %d, %d, %d, %d\\n", param1, param2/Parameters: %d, %d, %d, %d\\n", param2, param1/' phase_b_doubleneg
mutate "doubleneg stdout: 'foo()' banner text changed" \
  's/=== Starting foo\(\) execution ===/=== Starting bar() execution ===/' phase_b_doubleneg

echo
echo "=== mutations that are provably behaviour-preserving (should survive) ==="

# (x % 256) as i8 == x.rem_euclid(256) as i8 : both keep the low 8 bits.
mutate_equiv "create: rem_euclid(256) [same low byte]" \
  's/\.wrapping_rem\(256\);\n        unsafe \{/.rem_euclid(256);\n        unsafe {/' phase_c_errors
# memchr converts its int argument to unsigned char, so the (char) narrowing
# in the C source is not observable.
mutate_equiv "find: skip the (char) narrowing [memchr re-narrows]" \
  's/let target: i8 = search_val as i8;/let target: i32 = search_val;/;s/target as c_int, size/target, size/' phase_c_errors
# The offset is always 0..=255, so %ld and %d render identically on x86-64.
mutate_equiv "doubleneg stdout: %ld -> %d [offset always 0..255]" \
  's/at offset: %ld/at offset: %d/' phase_b_doubleneg
# Rust's `as i32` already saturates to i32::MIN below -2^31, which coincides with
# the x86-64 cvttsd2si indefinite value, so the explicit lower-bound branch is
# unobservable (verified exhaustively around both endpoints + 20M random f64).
mutate_equiv "convert: lower bound <= -2147483649.0 [saturation covers it]" \
  's/truncated < -2147483648\.0/truncated <= -2147483649.0/' phase_c_errors
mutate_equiv "convert: drop the lower-bound branch entirely [saturation covers it]" \
  's/ \|\| truncated < -2147483648\.0//' phase_c_errors

revert
rebuild
echo
echo "=== mutation summary ==="
echo "  real divergences caught      : $caught"
echo "  equivalent mutants survived  : $survived"
echo "  unexpected results (failures): $bad"
if cmp -s "$ORIG" "$SRC"; then echo "  src/lib.rs restored: yes"; else echo "  src/lib.rs restored: NO (WARNING)"; fi
[ "$bad" -eq 0 ]
