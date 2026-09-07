# ERRORS.md — Error-surface table

Mechanically derived by grepping `c_src/src/lib.c` and `c_src/include/lib.h`
for every rejection mechanism:

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|if|switch|<|>|==|!=|MIN|MAX|errno|abort|exit' c_src/src/lib.c
3:tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth) {
4:    return 18U + channels +
5:           (((blocksize * bitdepth * (channels * (channels != 2))) +
6:             (blocksize * bitdepth * (channels == 2)) +
7:             (blocksize * (bitdepth + (bitdepth != 32)) * (channels == 2)) +
```

Findings:

* `return` statements: exactly **1**, and it is the unconditional success
  return of a computed value. There is no error return.
* `assert` / `RETURN_ERROR` / `abort` / `exit` / `errno`: **0 occurrences**.
* `NULL` / pointer arguments: **0** — all three parameters are `uint32_t` by
  value, so there is no null-pointer or out-of-bounds surface at all.
* `if` / `switch` / early-out statements: **0**. The `!=` / `==` occurrences on
  lines 5–7 are *arithmetic* comparison operators used as 0/1 multipliers, not
  rejections.
* Range checks, min/max constants, enums: **0**. `18U`, `7`, `8`, `2`, `32` are
  arithmetic constants, not validity bounds.
* Return type is `tflac_u32` (unsigned), so there is no `-1` sentinel space and
  no `NULL` sentinel space.

## Rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | `max_size_frame` | *(none — the C function has no rejection path)* | n/a |

**The error surface is empty by construction.** `max_size_frame` is a total
function on `(u32, u32, u32)`: every one of the 2^96 input triples is "valid"
and produces a defined `u32` (unsigned arithmetic wraps modulo 2^32; the only
division is by the literal `8`, so division-by-zero is impossible).

## Generic boundaries covered anyway (Phase C tests)

Because the table has no rows, Phase C instead exhaustively covers the generic
boundaries that could still make C and Rust diverge. Each is a differential
test asserting *identical* returned `u32` (the function's only observable
result), not merely "both failed":

| # | boundary class | inputs exercised | test |
|---|----------------|------------------|------|
| E1 | zero arguments | every subset of `{blocksize, channels, bitdepth}` set to `0` (all 8 combinations) | `err_zero_arguments` |
| E2 | `u32::MAX` / oversized lengths | every subset set to `u32::MAX` (all 27 combos over `{0,1,MAX}`) | `err_extremes_cross_product` |
| E3 | one step past the "documented" ranges | `channels` ∈ {1,2,3} (past the `==2` special case), `bitdepth` ∈ {31,32,33} (past the `!=32` special case), `blocksize` ∈ {0,1,65535,65536} | `err_one_past_range` |
| E4 | out-of-range "enum-like" values across FFI | `channels` and `bitdepth` given values with no musical meaning (0, 7, 255, 256, 0x7FFF_FFFF, 0x8000_0000, `u32::MAX`) — C accepts any `int`/`uint32_t` bit pattern | `err_out_of_range_enum_like` |
| E5 | multiplication overflow (wraparound) | triples chosen so `blocksize * bitdepth * channels` exceeds 2^32, incl. exact 2^32 multiples and `+7` carry at the wrap point | `err_overflow_wraparound` |
| E6 | sign-bit / high-bit values | args with bit 31 set, so a signed mis-translation would differ | `err_high_bit_values` |
| E7 | exhaustive small domain | all `blocksize,channels,bitdepth ∈ [0,64]^3` (274 625 triples) | `err_exhaustive_small_cube` |

## Status

All rows in the rejection table: **0 rows exist** (nothing to check — proven by
the greps above, not assumed). All generic-boundary rows **E1–E7 pass** against
both `.so`s (`cargo test --release`, tests `err_*`), plus `deep_stress_50m`
(50 000 000 random triples + exhaustive `(channels, bitdepth) ∈ [0,300]²`
against 8 blocksizes) passes with 0 divergences.

The differential harness was validated with a **negative control**: changing
the Rust `18u32` constant to `19u32` made `config_row_01` fail with
`C .so returned 20 ... Rust .so returned 21`, confirming the tests can actually
detect divergence. The injected change was reverted.
