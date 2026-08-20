# ERRORS.md — Phase A.2: error-surface table

## Mechanical derivation

Every rejection mechanism a C library can have was grepped for across the whole
C source (`c_src/src/lib.c`, `c_src/include/lib.h`):

```sh
grep -nE 'return -1|return NULL|RETURN_ERROR|assert|abort|exit\(|errno|_ERROR|goto|if ?\(|switch|#if|MIN|MAX|<=|>=' \
     src/lib.c include/lib.h
#   -> no matches (exit status 1)

grep -n 'return' src/lib.c include/lib.h
#   -> src/lib.c:4:    return 18U + channels +      (the single return, unconditional)

grep -nE '[*]|enum|struct|union' src/lib.c include/lib.h
#   -> only the `*` multiplication operators on lines 5-7; no pointers, no enums,
#      no structs, no unions
```

**Result: the C library contains ZERO rejection sites.** There is no
error-return macro, no `return -1`/`return NULL`, no error enum, no `assert`,
no explicit range check, no null check, and no min/max constant. `max_size_frame`
is a *total* function: exactly one unconditional `return` of a pure arithmetic
expression over three `uint32_t` values.

That is itself the property under test, and it is a strong one. The rows below
are therefore "**must NOT reject**" rows: each constructs an input that a
defensively-written library *would* plausibly reject, and asserts that C and
Rust both accept it **and agree bit-for-bit on the wrapped `u32` result**. A
Rust translation that added a bounds check, a `debug_assert!`, a `panic!`, or
that overflow-panicked in a debug build would fail these rows — which is exactly
the divergence class that matters here.

### Why the usual generic boundary classes are N/A (and what replaces them)

| generic class the prompt asks for | applicability | how it is covered instead |
|---|---|---|
| null pointers | **N/A** — the ABI is `(u32, u32, u32) -> u32`; there is no pointer parameter to nullify (verified by the `[*]` grep above) | row 20 passes the all-zero-bits argument triple, the ABI-level equivalent of "null" |
| zero / oversized lengths | **N/A** — there is no buffer, length, or count parameter and no memory is touched | rows 2–7 (zero-valued args) and rows 8–13 (`UINT32_MAX` args) cover the smallest and largest possible magnitudes |
| one step past a documented valid range | the only range-like constants in the source are the literals `2` (compared against `channels`) and `32` (compared against `bitdepth`) | rows 14–17 test `channels` = 1/2/3 and `bitdepth` = 31/32/33 |
| out-of-range enum values across FFI | **N/A** — no parameter is an enum | row 19 replaces it: `u32` accepts *every* one of its 2^32 bit patterns, so hostile patterns plus uniform-random values are swept; no bit pattern is invalid and none may be rejected. `tests/exhaustive_axis.rs` additionally enumerates **all 2^32** values of each axis in turn |

## Error-surface table

Legend for the expected-result column: `f(bs, ch, bd)` is the C function;
all arithmetic is mod 2^32. Expected values were confirmed against an
independent Python oracle *and* the compiled C `.so`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `max_size_frame` | Any input whatsoever — the source has no rejection site, so no input can be refused | Always returns; never sets `errno`, never aborts. Return value is the arithmetic expression, mod 2^32 |
| 2 | `max_size_frame` | `blocksize == 0` (degenerate "empty frame"), arbitrary `channels`/`bitdepth` | Accepted. All three products vanish; `(0+7)/8 == 0`, so result `== 18 + channels` (mod 2^32) |
| 3 | `max_size_frame` | `channels == 0` (degenerate "no channels"), arbitrary `blocksize`/`bitdepth` | Accepted. `channels * (channels != 2)` is `0` and `(channels == 2)` is `0`, so every term is `0`; result `== 18` exactly |
| 4 | `max_size_frame` | `bitdepth == 0` with `channels == 2` | Accepted. `T1 = T2 = 0`, but `T3 = blocksize * (0 + 1) * 1 = blocksize`; result `== 20 + (blocksize + 7)/8` |
| 5 | `max_size_frame` | `bitdepth == 0` with `channels != 2` | Accepted. All terms `0`; result `== 18 + channels` |
| 6 | `max_size_frame` | All three arguments `0` | Accepted. `f(0,0,0) == 18` |
| 7 | `max_size_frame` | `blocksize == 0 && channels == 0 && bitdepth == UINT32_MAX` | Accepted; result `== 18` |
| 8 | `max_size_frame` | `channels == UINT32_MAX` — makes the *outer* `18U + channels` overflow | Accepted, wraps. `f(0, 0xFFFFFFFF, 0) == 17` (`18 + 0xFFFFFFFF` mod 2^32 `== 17`) |
| 9 | `max_size_frame` | `bitdepth == UINT32_MAX` with `channels == 2` — makes the *inner* `bitdepth + (bitdepth != 32)` overflow to `0` | Accepted, wraps. `T3` collapses to `0` because `0xFFFFFFFF + 1 == 0`. `f(1, 2, 0xFFFFFFFF) == 20` |
| 10 | `max_size_frame` | `blocksize == UINT32_MAX`, arbitrary other args | Accepted; products wrap mod 2^32 |
| 11 | `max_size_frame` | All three arguments `UINT32_MAX` | Accepted; fully wrapped result, no trap |
| 12 | `max_size_frame` | Products overflow 32 bits, e.g. `blocksize == 0x10000, bitdepth == 0x10000, channels == 1` (`T1 == 2^32 -> 0`) | Accepted; truncated to low 32 bits before the divide |
| 13 | `max_size_frame` | Term sum lands so that `sum + 7` itself overflows (e.g. `sum == 0xFFFFFFFF` via `f(1, 2, 0xFFFFFFFF)`, `sum + 7 == 6`) | Accepted; the wrapped small numerator is divided, giving `6/8 == 0` — **not** a saturated/rounded-up value |
| 14 | `max_size_frame` | `channels == 1` — one below the special-cased `2` | Accepted; takes the `channels != 2` path (`T1` only) |
| 15 | `max_size_frame` | `channels == 3` — one above the special-cased `2` | Accepted; takes the `channels != 2` path (`T1` only) |
| 16 | `max_size_frame` | `bitdepth == 31` and `bitdepth == 33` — one step either side of the special-cased `32` | Accepted; `(bitdepth != 32)` is `1`, so `T3` uses `bitdepth + 1` |
| 17 | `max_size_frame` | `bitdepth == 32` exactly — the sole equality-guarded value | Accepted; `(bitdepth != 32)` is `0`, so `T3` uses `bitdepth` unchanged |
| 18 | `max_size_frame` | `channels == 2` exactly — the sole equality-guarded value for channels | Accepted; `T1` is forced to `0` and `T2 + T3` become active |
| 19 | `max_size_frame` | Arbitrary "invalid" 32-bit patterns in all three args (stand-in for out-of-range enum values: a `u32` parameter has no invalid variant, so every bit pattern must be handled) | Accepted; no bit pattern may be rejected. Covered by a 76-value hostile-pattern cross product (76^3 calls: all-ones, alternating, every single-bit and its complement, sign bit) **plus** 200 000 uniform-random triples. A genuinely exhaustive per-axis 2^32 sweep is in `tests/exhaustive_axis.rs` |
| 20 | `max_size_frame` | All-zero-bits argument triple (ABI analogue of passing `NULL`) | Accepted; `== 18` (same as row 6) |
| 21 | `max_size_frame` | Callee must not clobber/mis-widen the ABI: values `> INT32_MAX` (`0x80000000`, `0xFFFFFFFE`) passed where a signed misinterpretation would flip sign | Accepted; treated as unsigned, no sign extension |

**Total rows: 21.**

## Row status

Each row is covered by a named `#[test]` in `tests/error_paths.rs` that calls
**both** `.so` files through `libloading` and asserts the two return values are
identical, and additionally asserts the concrete expected value where the table
states one. Rows are checked off only after that test passes against both
libraries.

| # | test | status |
|---|------|--------|
| 1 | `row01_function_is_total_never_rejects` | [x] |
| 2 | `row02_blocksize_zero` | [x] |
| 3 | `row03_channels_zero` | [x] |
| 4 | `row04_bitdepth_zero_stereo` | [x] |
| 5 | `row05_bitdepth_zero_non_stereo` | [x] |
| 6 | `row06_all_args_zero` | [x] |
| 7 | `row07_zero_zero_max` | [x] |
| 8 | `row08_channels_uint32_max_outer_overflow` | [x] |
| 9 | `row09_bitdepth_uint32_max_inner_overflow` | [x] |
| 10 | `row10_blocksize_uint32_max` | [x] |
| 11 | `row11_all_args_uint32_max` | [x] |
| 12 | `row12_product_overflow` | [x] |
| 13 | `row13_numerator_plus_seven_overflow` | [x] |
| 14 | `row14_channels_one_below_two` | [x] |
| 15 | `row15_channels_one_above_two` | [x] |
| 16 | `row16_bitdepth_adjacent_to_32` | [x] |
| 17 | `row17_bitdepth_exactly_32` | [x] |
| 18 | `row18_channels_exactly_2` | [x] |
| 19 | `row19_arbitrary_bit_patterns_no_invalid_variant` | [x] |
| 20 | `row20_all_zero_bits_null_analogue` | [x] |
| 21 | `row21_values_above_int32_max_no_sign_extension` | [x] |

All 21 rows checked. See `VERIFICATION.md` for the final gate.
