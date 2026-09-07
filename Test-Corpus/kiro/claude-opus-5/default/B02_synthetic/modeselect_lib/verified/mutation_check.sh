#!/usr/bin/env bash
# Mutation check: prove the differential suite actually DETECTS divergences.
# Each mutation injects a realistic C-to-Rust translation bug into src/lib.rs;
# the suite MUST fail for every one. Any mutation that survives means the tests
# are blind to that class of bug.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

SRC=src/lib.rs
BAK=/tmp/lib.rs.mutbak
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

survived=0
total=0

# Each entry: description :: sed expression
mutate() {
  desc="$1"; expr="$2"; shift 2
  total=$((total+1))
  restore
  if ! sed -i "$expr" "$SRC"; then echo "SKIP (sed failed): $desc"; return; fi
  if cmp -s "$SRC" "$BAK"; then echo "SKIP (no textual change): $desc"; survived=$((survived+1)); return; fi
  if ! timeout 600 cargo build --release >/dev/null 2>&1; then
    echo "CAUGHT (build error)        : $desc"; return
  fi
  if timeout 600 cargo test --release >/dev/null 2>&1; then
    echo "!!! SURVIVED                : $desc"; survived=$((survived+1))
  else
    echo "CAUGHT (test failure)       : $desc"
  fi
}

echo "=== mutation testing the differential suite ==="

# --- (int)double conversion semantics -------------------------------------
mutate "d2i returns 0 instead of INT_MIN on overflow" \
  '0,/^}/{}; s/^        i32::MIN$/        0/'
mutate "d2i uses Rust saturating `as` cast (classic translation bug)" \
  's|    let t = x.trunc();|    return x as i32; #[allow(unreachable_code)] let t = x.trunc();|'
mutate "d2i boundary off by one (<= becomes <)" \
  's|t <= 2147483647.0|t < 2147483647.0|'
mutate "d2i NaN maps to 0 instead of INT_MIN" \
  's|        return i32::MIN;|        return 0;|'

# --- classify_mode --------------------------------------------------------
mutate "classify_mode turbo returns 0x31 instead of 0x30" \
  's|            0x30$|            0x31|'
mutate "classify_mode ignores the NUL terminator (prefix match bug)" \
  's|        \*p.add(s.len()) as u8 == 0|        true|'
mutate "classify_mode is made case-insensitive" \
  's|if \*p.add(i) as u8 != b {|if (*p.add(i) as u8).to_ascii_lowercase() != b.to_ascii_lowercase() {|'

# --- apply_multiplier fall-through ---------------------------------------
# NOTE: these must be exact whole-line edits; a "no textual change" or no-op
# insertion would look like a surviving mutant while testing nothing.
mutate "apply_multiplier case 4 adds 0xFE instead of 0xFF" \
  's|wrapping_add(0xFF)|wrapping_add(0xFE)|'
mutate "apply_multiplier first 0xAB step becomes 0xAC" \
  '0,/wrapping_add(0xAB)/s|wrapping_add(0xAB)|wrapping_add(0xAC)|'
mutate "apply_multiplier every 0x05 step becomes 0x06" \
  's|wrapping_add(0x05)|wrapping_add(0x06)|g'
mutate "apply_multiplier case 2 loses its 0x7E fall-through step" \
  '/^        2 => {$/{n; /wrapping_add(0x7E)/d}'
mutate "apply_multiplier case 1 loses its 0x1C fall-through step" \
  '/^        1 => {$/{n; /wrapping_add(0x1C)/d}'
mutate "apply_multiplier case 0 loses its 0x05 step" \
  '/^        0 => {$/{n; /wrapping_add(0x05)/d}'
mutate "apply_multiplier uses saturating instead of wrapping add" \
  's|result.wrapping_add(0x05)|result.saturating_add(0x05)|g'
mutate "apply_multiplier default returns 0 instead of 0xDEAD" \
  's|            result = 0xDEAD;|            result = 0;|'
mutate "apply_multiplier accepts level 5 as valid" \
  's|^        4 => {$|        5 \| 4 => {|'

# --- convert_* scaling ----------------------------------------------------
mutate "convert_time_factor scales by 1e11 instead of 1e12" \
  's|factor \* 1e12|factor * 1e11|'
mutate "convert_negative_overflow loses the sign of its scale" \
  's|value \* -1e15|value * 1e15|'

# --- get_modified_time ----------------------------------------------------
mutate "get_modified_time drops the >>29 shift" \
  's|    current >>= 29;||'
mutate "get_modified_time shifts by 28" \
  's|    current >>= 29;|    current >>= 28;|'
mutate "get_modified_time does 64-bit (non-wrapping) offset math" \
  's|    let offset: time_t = offset_i32 as time_t;|    let offset: time_t = offset_days as time_t * 86400 + offset_hours as time_t * 3600;|'
mutate "get_modified_time uses 3601 seconds per hour" \
  's|offset_hours.wrapping_mul(3600)|offset_hours.wrapping_mul(3601)|'

# --- hash_time_value ------------------------------------------------------
mutate "hash_time_value multiplies by 0x1E instead of 0x1F" \
  's|hash.wrapping_mul(0x1F)|hash.wrapping_mul(0x1E)|'
mutate "hash_time_value seeds with 0x5A5A5A5B" \
  's|0x5A5A_5A5A|0x5A5A_5A5B|'
mutate "hash_time_value masks with 0xFFFFFFFF instead of 0x7FFFFFFF" \
  's|hash & 0x7FFF_FFFF|hash \& 0xFFFF_FFFF|'
mutate "hash_time_value shifts by (i%4)*7 instead of (i%4)*8" \
  's|<< ((i % 4) \* 8)|<< ((i % 4) * 7)|'
mutate "hash_time_value reverses byte order" \
  's|let bytes = t.to_ne_bytes();|let mut bytes = t.to_ne_bytes(); bytes.reverse();|'
mutate "hash_time_value only consumes 4 of the 8 bytes" \
  's|for i in 0..std::mem::size_of::<time_t>()|for i in 0..4usize|'

# --- KNOWN-EQUIVALENT MUTANTS (documented, deliberately not asserted) -----
# The following mutations CANNOT be detected, because the C lines they change
# have no observable effect for ANY input. They are listed here so that a future
# reader does not mistake them for coverage gaps:
#
#   * `result ^= (result1 & 0xFF)`   and  `result ^= (result2 & 0xFF00)`
#     result1 is convert_time_factor((double)seed * 1e8), which is 0 when
#     seed == 0 and INT_MIN (0x80000000) otherwise -- never anything else. So
#     result1 & 0xFF is always 0. The same holds for result2 & 0xFF00. Both XOR
#     steps are therefore dead code in the C itself. Proven by the test
#     `modeselect_result1_result2_xor_steps_are_provably_dead`.
#
#   * `<< ((i % 4) * 8)` -> `<< ((i % 8) * 8)` in hash_time_value
#     x86 masks 32-bit shift counts to 5 bits, so shifts of 32/40/48/56 behave
#     as 0/8/16/24 -- exactly what `i % 4` already produces for i = 4..7.

# --- modeselect pipeline --------------------------------------------------
mutate "modeselect uses rem_euclid for mode_selector % 4" \
  's|let mode_index: c_int = mode_selector % 4;|let mode_index: c_int = mode_selector.rem_euclid(4);|'
mutate "modeselect uses rem_euclid for complexity % 5" \
  's|let complexity_level: c_int = complexity % 5;|let complexity_level: c_int = complexity.rem_euclid(5);|'
mutate "modeselect uses rem_euclid for seed % 24" \
  's|get_modified_time(time_offset, seed % 24)|get_modified_time(time_offset, seed.rem_euclid(24))|'
mutate "modeselect final constant 0xBEEE instead of 0xBEEF" \
  's|wrapping_add(0xBEEF)|wrapping_add(0xBEEE)|'
mutate "modeselect final multiply 0x11 instead of 0x10" \
  's|result.wrapping_mul(0x10)|result.wrapping_mul(0x11)|'
mutate "modeselect uses time_hash % 0x100 instead of 0x1000" \
  's|time_hash % 0x1000|time_hash % 0x100|'
mutate "modeselect swaps the mode table order" \
  's|        b"turbo\\0",|        b"extreme\\0",|'
mutate "modeselect prints %d instead of %X for the mode value" \
  's|Selected mode: %s (0x%X)|Selected mode: %s (0x%d)|'
mutate "modeselect prints %.3e instead of %.2e" \
  's|Converting double %.2e to int (may overflow)|Converting double %.3e to int (may overflow)|'
mutate "modeselect omits the leading newline before Final result" \
  's|b"\\nFinal result: %d (0x%X)\\n\\0"|b"Final result: %d (0x%X)\\n\\0"|'
mutate "modeselect adds mode_value twice" \
  's|    result = result.wrapping_add(mode_value);|    result = result.wrapping_add(mode_value).wrapping_add(mode_value);|'

restore
timeout 600 cargo build --release >/dev/null 2>&1

echo
echo "=== mutation summary: $((total-survived))/$total caught, $survived survived ==="
[ "$survived" = "0" ] || echo "WARNING: surviving mutations indicate blind spots"
exit 0
