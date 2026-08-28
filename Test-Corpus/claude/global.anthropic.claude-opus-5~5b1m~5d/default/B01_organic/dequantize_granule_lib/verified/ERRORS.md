# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Grep results for every rejection
construct in the library:

```
$ grep -nE 'return|assert|NULL|<|>|\?' c_src/src/lib.c
7:    if ((bs->pos += n) > bs->limit)     <-- the ONLY explicit rejection
8:        return 0;                       <-- its error value (a sentinel, not a code)
14:    return cache | (next >> -shl);
20:    for (j = 0; j < 4; j++)             <-- loop guards (implicit rejections)
22:    for (i = 0; i < 2 * sci->total_bands; i++)
24:    if (ba != 0)
25:    if (ba < 17)
27:    for (k = 0; k < group_size; k++)
33:    for (k = 0; k < group_size; k++, ...)
42:    return group_size * 4;
```

Facts this establishes:

* There is **no** `RETURN_ERROR` macro, **no** error enum, **no** `assert`,
  **no** null-pointer check and **no** range check anywhere in the library.
* `dequantize_granule` has exactly one `return` and it is unconditional:
  `return group_size * 4;`. It can therefore never report failure.
* The single genuine error path is the bitstream-underrun guard in `get_bits`,
  whose "error value" is the sentinel `0`. It is *not* observable directly
  (`get_bits` is `static`); it is observable only through the float values
  written into `grbuf`, and through the fact that `bs->pos` is advanced
  **before** the guard fires (so the underrun is sticky/latching).
* Every other rejection is a *loop guard* that silently degenerates to
  "do nothing" (empty output), which is the library's way of rejecting
  degenerate sizes.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result | differential test | [x] |
|----|----------|----------------------------------------------|-------------------|-------------------|-----|
| E1 | `get_bits` (via `dequantize_granule`, `ba < 17` path) | `bs->pos + n > bs->limit` — bitstream underruns mid-granule | `get_bits` returns sentinel `0`; `dst[k] = (float)(0 - half) = -half`; `bs->pos` **is still advanced** by `n`; `dequantize_granule` still returns `group_size * 4` | `e1_underrun_in_half_branch_yields_minus_half` | [x] |
| E2 | `get_bits` (via `dequantize_granule`, `ba >= 17` path) | `bs->pos + n > bs->limit` | sentinel `0` ⇒ `code == 0` ⇒ every `dst[k] = (float)(int)(0 % mod - mod/2) = -(mod/2)`; `bs->pos` advanced; return `group_size * 4` | `e2_underrun_in_mod_branch_yields_minus_mod_over_two` | [x] |
| E3 | `get_bits` | `bs->limit == 0`, `bs->pos == 0` (empty stream) | first call already underruns (`0 + n > 0` for all `n >= 1`); *all* reads return `0` for the rest of the call | `e3_limit_zero_with_pos_zero_empty_stream` | [x] |
| E4 | `get_bits` | `bs->limit < 0` (negative limit) | comparison is signed, so `pos + n > limit` for any non-negative `pos`; every read returns `0` | `e4_negative_limit` | [x] |
| E5 | `get_bits` | `bs->pos > bs->limit` already on entry (exhausted stream) | every read returns `0`; `pos` keeps being advanced past `limit` (monotonically, latching) | `e5_pos_already_past_limit_is_latching` | [x] |
| E6 | `get_bits` | `bs->pos == bs->limit` on entry, `n >= 1` | `pos + n > limit` ⇒ `0`. Boundary: one step past the valid range | `e6_pos_exactly_equals_limit_one_step_past_valid_range` | [x] |
| E7 | `get_bits` | `bs->pos + n == bs->limit` exactly (last legal read) | guard is `>` not `>=`, so this is **accepted** — the off-by-one boundary must not reject | `e7_pos_plus_n_exactly_equals_limit_is_accepted` | [x] |
| E8 | `get_bits` | `bs->pos < 0` (negative bit position) with `limit < pos` | guard fires first ⇒ returns `0` **before** dereferencing `bs->buf + (pos >> 3)`; no out-of-bounds read. (With `limit >= pos + n` the C reads out of bounds — genuine UB, not a rejection, therefore not a differential test case.) | `e8_negative_pos_guard_fires_before_any_dereference` | [x] |
| E9 | `dequantize_granule` | `group_size == 0` | both `k` loops have zero iterations ⇒ `grbuf` is never written; but in the `ba >= 17` branch `get_bits` is called **before** the `k` loop, so `bs->pos` still advances. Returns `0` | `e9_group_size_zero_still_consumes_mod_branch_bits` | [x] |
| E10 | `dequantize_granule` | `group_size < 0` (negative size) | `k < group_size` is false immediately ⇒ no writes; `dst = grbuf + group_size*j` points *before* `grbuf` but is never dereferenced; returns `group_size * 4` (negative, wrapping) | `e10_negative_group_size_writes_nothing_and_returns_negative` | [x] |
| E11 | `dequantize_granule` | `sci->total_bands == 0` | `i < 2*0` false ⇒ inner loop never runs, `bs` never touched, `grbuf` never written; returns `group_size * 4` | `e11_total_bands_zero_touches_nothing` | [x] |
| E12 | `dequantize_granule` | `sci->bitalloc[i] == 0` for a band | `if (ba != 0)` rejects the band: no `get_bits`, no write, but `dst += choff` / `choff = 18 - choff` still happen (band-position bookkeeping still advances) | `e12_zero_bitalloc_skips_band_but_still_walks_choff` | [x] |
| E13 | `dequantize_granule` | `grbuf == NULL` combined with a condition that writes nothing (`group_size <= 0`, or `total_bands == 0`, or all `bitalloc[i] == 0`) | pointer arithmetic on `NULL` only; no dereference; returns `group_size * 4` | `e13_null_grbuf_when_nothing_is_written` | [x] |
| E14 | `dequantize_granule` | `bs == NULL` with `total_bands == 0` or all `bitalloc == 0` | `get_bits` never called ⇒ `bs` never dereferenced; returns `group_size * 4` | `e14_null_bs_when_get_bits_is_never_called` | [x] |
| E15 | `dequantize_granule` | `sci->total_bands > 32` ⇒ `i` reaches 64.. ⇒ `sci->bitalloc[i]` reads **past** the 64-byte `bitalloc` array into `scfcod` | no check exists; C happily reads the adjacent struct bytes and uses them as bit allocations | `e15_total_bands_above_32_reads_past_bitalloc_into_scfcod` | [x] |
| E16 | `dequantize_granule` | `sci->total_bands > 64` (up to 255) ⇒ `i` up to 509 ⇒ reads past the end of `L12_scale_info` entirely | no check exists; C reads whatever follows the struct | `e16_total_bands_above_64_reads_past_the_whole_struct` | [x] |
| E17 | `dequantize_granule` | out-of-range "opcode": `bitalloc[i] == 17` (first value of the second, `mod`-coded branch) | takes the `else` branch: `mod = (2 << 0) + 1 = 3`, `n = 3 + 2 - 0 = 5` | `e17_to_e22_every_bitalloc_opcode_value` | [x] |
| E18 | `dequantize_granule` | out-of-range "opcode": `bitalloc[i] == 16` (last value of the `half` branch) | `half = (1 << 15) - 1 = 32767`, reads 16 bits | `e18_ba16_is_the_last_half_branch_value_and_e17_ba17_the_first_mod_value` | [x] |
| E19 | `dequantize_granule` | out-of-range "opcode": `bitalloc[i] == 48` ⇒ `ba - 17 == 31` ⇒ `2 << 31` shifts a 32-bit `int` by 31 ⇒ `0` ⇒ `mod == 1` | `code % 1 - 1/2 == 0` ⇒ every `dst[k] = 0.0`; `n = 1 + 2 - 0 = 3` | `e19_ba48_shift_by_31_gives_mod_one_and_all_zero_samples` | [x] |
| E20 | `dequantize_granule` | out-of-range "opcode": `bitalloc[i] == 47` ⇒ `2 << 30` overflows `int` to `0x80000000` | `mod = (unsigned)(INT_MIN + 1) = 0x80000001`, `n = (int)(0x80000001 + 2 - 0x10000000) = 0x70000003` ⇒ huge `n` ⇒ E1/E2 underrun guard fires ⇒ `dst[k] = -(mod/2) = -(0x40000000) = -1073741824` | `e20_ba47_signed_shift_overflow_mod_is_0x80000001` | [x] |
| E21 | `dequantize_granule` | out-of-range "opcode": `bitalloc[i] == 49` ⇒ `ba - 17 == 32` ⇒ shift count masked to 5 bits ⇒ behaves exactly like `ba == 17` | wrap-around aliasing with period 32 must be reproduced | `e21_shift_count_masking_makes_ba_alias_with_period_32` | [x] |
| E22 | `dequantize_granule` | out-of-range "opcode": `bitalloc[i] == 255` (max `uint8_t`) ⇒ `(255-17) & 31 == 14` ⇒ `mod = 32769`, `n = 28675` | very wide read; underruns unless `limit` is enormous | `e22_ba255_max_uint8_value` | [x] |
| E23 | `get_bits` | `n` large enough that `shl = n + s` needs many 8-bit steps but `pos + n <= limit` (wide legal read, e.g. `ba == 31`, `n == 28675`) | the `while` loop runs `ceil((n+s)/8)-1` times, OR-ing shifted bytes; only the **low 32 bits** survive because `cache` is `uint32_t` and `next << shl` has its shift count masked to 5 bits | `e23_very_wide_legal_read_only_low_32_bits_survive` | [x] |
| E24 | `get_bits` | `bs->pos & 7 != 0` (unaligned start) with `n` such that `shl` ends at exactly `0` ⇒ final `next >> -shl` is `next >> 0` | must not be treated as a shift-by-32 | `e24_final_shift_is_by_zero_not_by_thirtytwo` | [x] |
| E25 | `dequantize_granule` | `grbuf` valid but too small for the `choff` walk (`dst` steps `+576`, `-558`, `+576`, …) | no check exists; C writes wherever `dst` lands. Differentially testable with a generously sized buffer: the *pattern of touched offsets* must match exactly | `e25_choff_walk_has_no_bounds_check` | [x] |

### Deliberately excluded from differential testing (undefined behaviour that
### crashes *both* implementations identically, so there is no observable result
### to compare)

| condition | why excluded |
|-----------|--------------|
| `sci == NULL` | `sci->total_bands` is dereferenced unconditionally ⇒ SIGSEGV in C and in Rust |
| `bs == NULL` while some `bitalloc[i] != 0` and `group_size > 0` | `bs->pos` dereferenced ⇒ SIGSEGV in both |
| `bs->limit == INT_MAX` together with `ba == 47` (`n == 0x70000003`) | the underrun guard does *not* fire, so C loops ~2.3·10⁸ times reading unmapped memory ⇒ SIGSEGV in both |
| `bs->pos` near `INT_MAX` with positive `n` | signed overflow makes `pos` negative, guard passes, `buf + (pos>>3)` reads unmapped memory ⇒ SIGSEGV in both |
| `bs->pos < 0` with `limit >= pos + n` | `buf + (pos >> 3)` reads before the buffer ⇒ SIGSEGV / garbage in both |

## Generic boundaries covered beyond the table

| boundary | differential test |
|----------|-------------------|
| null `grbuf` on every non-writing path | `e13_null_grbuf_when_nothing_is_written` |
| null `bs` on every path that never reads bits | `e14_null_bs_when_get_bits_is_never_called` |
| zero length (`group_size == 0`, `total_bands == 0`) | `e9_group_size_zero_still_consumes_mod_branch_bits`, `e11_total_bands_zero_touches_nothing` |
| negative length (`group_size < 0`) | `e10_negative_group_size_writes_nothing_and_returns_negative` |
| oversized length: `total_bands` = every value 0..=255 | `every_total_bands_value_zero_through_255` |
| `group_size` at `INT_MIN`, `INT_MIN+1`, `INT_MAX-1`, `INT_MAX`, `±2^30` (wrapping `group_size * 4`) | `extreme_group_size_values_with_no_inner_loop` |
| `bs->pos` / `bs->limit` at `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `7`, `8`, `INT_MAX-1`, `INT_MAX` (all 81 pairs) | `extreme_pos_and_limit_values` |
| out-of-range "enum" across the FFI boundary: `bitalloc` byte = every value 0..=255, i.e. every value with no valid MPEG variant (only 0..=16 are legal) | `e17_to_e22_every_bitalloc_opcode_value` |
| one step past the documented range on both sides: `ba == 16` / `ba == 17`, `pos + n == limit` / `== limit - 1`, `pos == limit` | `e18_...`, `e7_...`, `e10_...`, `e6_...` |

## Result

All **25** rows plus every generic boundary above pass against both the debug
and the release Rust `.so`, and against the C `.so` built at `-O0` and `-O2`.

```
cargo test --offline --test phase_c
```
