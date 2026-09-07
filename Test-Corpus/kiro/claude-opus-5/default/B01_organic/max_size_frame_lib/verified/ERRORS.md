# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source, not from docs or assumptions.

## Mechanical grep evidence

Run against `c_src/include/lib.h` and `c_src/src/lib.c`:

| pattern searched | occurrences |
|------------------|-------------|
| `return` statements | 1 (the single unconditional `return` of the result expression) |
| `RETURN_ERROR` / error macros | 0 |
| `assert` | 0 |
| `NULL` / pointer parameters | 0 (all three parameters are by-value `tflac_u32`) |
| `errno` | 0 |
| `goto` | 0 |
| `if` / `switch` / `?:` statements | 0 |
| `#if` / `#ifdef` | 0 |
| explicit range / bounds checks | 0 |
| `MAX` / `MIN` / `LIMIT` constants | 0 |
| division by a non-constant | 0 (the only division is `/ 8`, so no div-by-zero is reachable) |
| allocation / I/O / other failable calls | 0 |

## The error surface

**The C function has NO error surface.** `max_size_frame` is a total function:
it accepts all 2^96 combinations of its three `uint32_t` arguments, performs
only wrapping unsigned arithmetic plus a division by the constant 8, and always
returns a `tflac_u32`. There is no sentinel value, no error code, no
out-parameter, and no way for it to reject input.

Consequently the "error path" that must match between C and Rust is precisely
this: **for every input the C would notionally consider invalid, it does not
reject — it returns a well-defined wrapped result, and the Rust must return the
same wrapped result rather than panicking, aborting, or saturating.** That is
what the rows below assert. A Rust `debug_assert`, an overflow panic, or a
saturating/checked substitution would be a divergence.

| # | function | trigger (the exact invalid/edge input or condition) | expected C result | test |
|---|----------|------------------------------------------------------|-------------------|------|
| E1 | `max_size_frame` | `blocksize = 0` (degenerate/empty block) | no rejection; returns `18 + channels + (0+7)/8` = `18 + channels` | `err_e1_blocksize_zero` |
| E2 | `max_size_frame` | `channels = 0` (nonsensical channel count) | no rejection; `channels != 2` is 1 but `channels * 1 == 0`, so term1 = 0; returns `18 + 0 + 7/8` = `18` | `err_e2_channels_zero` |
| E3 | `max_size_frame` | `bitdepth = 0` (nonsensical bit depth) | no rejection; for `channels != 2` returns `18 + channels + 0`; for `channels == 2` returns `18 + 2 + (blocksize*1 + 7)/8` because `(bitdepth != 32)` is 1 | `err_e3_bitdepth_zero` |
| E4 | `max_size_frame` | all three arguments `0` | no rejection; returns `18` | `err_e4_all_zero` |
| E5 | `max_size_frame` | `blocksize = UINT32_MAX` (oversized length) | no rejection; wrapping multiply, no trap | `err_e5_blocksize_max` |
| E6 | `max_size_frame` | `channels = UINT32_MAX` (oversized count, one past every valid range) | no rejection; wrapping multiply and wrapping `18 + channels`, no trap | `err_e6_channels_max` |
| E7 | `max_size_frame` | `bitdepth = UINT32_MAX` (oversized depth) | no rejection; `bitdepth + 1` wraps to `0` inside term3, no trap | `err_e7_bitdepth_max` |
| E8 | `max_size_frame` | all three arguments `UINT32_MAX` | no rejection; fully wrapped result | `err_e8_all_max` |
| E9 | `max_size_frame` | `bitdepth = 32` exactly (the boundary the `bitdepth != 32` test keys on) with `channels = 2` | no rejection; term3 uses `bitdepth + 0`, i.e. the `+1` correction is suppressed | `err_e9_bitdepth_32_boundary` |
| E10 | `max_size_frame` | `bitdepth = 31` and `bitdepth = 33` (one step either side of the 32 boundary) with `channels = 2` | no rejection; `+1` correction IS applied on both sides | `err_e10_bitdepth_off_by_one` |
| E11 | `max_size_frame` | `channels = 2` exactly (the boundary both `channels` tests key on) | no rejection; term1 suppressed, terms 2+3 active | `err_e11_channels_2_boundary` |
| E12 | `max_size_frame` | `channels = 1` and `channels = 3` (one step either side of the 2 boundary) | no rejection; term1 active, terms 2+3 suppressed | `err_e12_channels_off_by_one` |
| E13 | `max_size_frame` | inputs chosen so the inner bit-count sum overflows 2^32 (`blocksize * bitdepth * channels > UINT32_MAX`), e.g. `blocksize = 0x10000, bitdepth = 0x100, channels = 0x100` | no rejection; result is the *wrapped* sum divided by 8, NOT a saturated or panicking value | `err_e13_inner_overflow` |
| E14 | `max_size_frame` | inputs chosen so the `+7` itself overflows (inner sum `>= UINT32_MAX - 6`) | no rejection; `+7` wraps past zero, so the division yields a *small* number | `err_e14_plus7_overflow` |
| E15 | `max_size_frame` | inputs chosen so the final `18 + channels + bytes` overflows | no rejection; final addition wraps | `err_e15_final_add_overflow` |
| E16 | `max_size_frame` | out-of-range "enum-like" integer passed for `channels` (values with no meaningful audio interpretation: 4, 8, 0xFF, 0x10000, 0x7FFFFFFF, 0x80000000) — C accepts any `uint32_t` | no rejection; each is just `channels != 2`, arithmetic proceeds | `err_e16_out_of_range_channel_values` |
| E17 | `max_size_frame` | out-of-range "enum-like" integer passed for `bitdepth` (values that are not real FLAC depths: 1, 7, 33, 64, 0xFFFF, 0x80000000) | no rejection; only the `== 32` test matters, arithmetic proceeds | `err_e17_out_of_range_bitdepth_values` |

## Notes on C semantics that the rows above pin down

* `uint32_t` is `unsigned int` on this ABI (verified: `sizeof(unsigned) == 4`
  and `__builtin_types_compatible_p(uint32_t, unsigned) == 1`), so the integer
  promotions do **not** widen any operand to `int`. Every operation is done in
  `unsigned int` and therefore wraps modulo 2^32 with fully defined behaviour —
  there is no signed-overflow UB to worry about, and equally no trap for the
  Rust to reproduce.
* `(channels != 2)`, `(channels == 2)`, `(bitdepth != 32)` are C `int`s valued
  0 or 1, converted to `unsigned int` in the surrounding multiplications.
* The stray unary `+` in `+7` is a no-op; the addend is 7.
* `/ 8` on an unsigned value truncates toward zero and cannot trap.

## Gate status

- [x] E1  passing
- [x] E2  passing
- [x] E3  passing
- [x] E4  passing
- [x] E5  passing
- [x] E6  passing
- [x] E7  passing
- [x] E8  passing
- [x] E9  passing
- [x] E10 passing
- [x] E11 passing
- [x] E12 passing
- [x] E13 passing
- [x] E14 passing
- [x] E15 passing
- [x] E16 passing
- [x] E17 passing

## Verification evidence

All 17 rows have a passing differential test in
`translation/tests/differential.rs` (test names in the last column above). Each
loads both `.so`s via `libloading` and compares the returned `u32` byte-for-byte.

Beyond "both agree", the rows marked with a documented C result also pin the
exact value the C produces, so the test would catch a case where C and Rust
agreed on a *wrong* value because both were changed:

* E2 — `channels == 0` returns exactly `18` for every `blocksize`/`bitdepth`.
* E4 — `(0, 0, 0)` returns exactly `18`.
* E14 — inner sum in `0xFFFF_FFF9..=0xFFFF_FFFF` returns exactly `19`, i.e. the
  `+7` wrapped past zero and the byte count collapsed to `0`. The immediately
  preceding value `0xFFFF_FFF8` returns `19 + 0x1FFF_FFFF`, confirming the wrap
  boundary is exactly where the C puts it and not one step away.
* E15 — `18 + (UINT32_MAX - 17)` wraps to exactly `0`; `18 + (UINT32_MAX - 16)`
  wraps to exactly `1`.
* E9/E10/E12 — the full expression is recomputed independently with
  `wrapping_*` and compared against the C's return value.

No divergence between C and Rust was found on any error-surface row. The one
test failure encountered during this phase was a miscalculated constant in the
E15 *test* (`18 + (MAX-17)` wraps to `0`, not `1`); the C and Rust libraries
agreed with each other throughout. The test expectation was corrected — the
translation was not changed.

### Confirmation that the Rust cannot panic where the C wraps

`translation/src/lib.rs` uses `wrapping_mul` / `wrapping_add` for every C
operator; the only non-wrapping operator is `bits / 8`, an unsigned division by
a nonzero constant, which cannot trap. Verified two ways:

* `grep` for non-wrapping arithmetic outside comments in `src/lib.rs` returns
  exactly one line: `let bytes = bits / 8;`.
* The **debug** cdylib (overflow checks enabled) contains no
  `attempt to add/subtract/multiply/divide ... with overflow` panic message for
  this function, and the full suite passes against the debug `.so` as well as
  the release one — so no latent overflow panic exists in the unoptimised build.
