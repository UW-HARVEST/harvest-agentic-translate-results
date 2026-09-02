# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep evidence

Run over `c_src/src` and `c_src/include`:

| pattern searched | hits |
|---|---|
| `return -`, `return NULL`, `RETURN_ERROR` | 0 |
| `assert` | 0 |
| `errno`, `ERROR`, `_ERR` | 0 |
| `goto`, `exit(`, `abort(` | 0 |
| `if`, `switch`, `for`, `while`, `?:` | 0 |
| `#ifdef`, `#if` | 0 |
| `return` (any) | 1 — `src/lib.c:4`, the single unconditional return |
| `*` used as pointer declarator/deref | 0 (all `*` are multiplications) |
| enum declarations | 0 |
| min/max named constants | 0 |

**The C library has no error surface.** `max_size_frame` is a total function: it
takes three `uint32_t` by value, performs only unsigned arithmetic, and returns
unconditionally. There is no rejection path, no sentinel return, no error code,
no assertion, no range check, and no null check to replicate — there are no
pointer parameters to null-check and no enum parameters to feed an out-of-range
variant.

Consequently the *classical* error-surface table has zero rows:

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | *(no rejection/error branch exists in the C source)* | — |

## Generic-boundary rows (tested anyway)

Because the C rejects nothing, every input is "valid" and every input has a
defined result. The generic boundaries the task requires are therefore recast as
"the C must not trap and the Rust must reproduce the same defined value". The two
ways a C-to-Rust translation of this expression can actually *fail* are (a) a Rust
arithmetic-overflow panic where C wraps, and (b) a division difference. Both are
covered below. `panic = "abort"` is set for the release profile, so an overflow
panic in the Rust `.so` would abort the differential test process — a hard,
visible failure.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `max_size_frame` | all-zero arguments `(0, 0, 0)` — degenerate/empty shape | no trap; returns `18 + 0 + (0+7)/8` = `18` | `err_e1_all_zero` | [x] |
| E2 | `max_size_frame` | `channels = 0` (zero "length"), `blocksize`/`bitdepth` nonzero | no trap; `channels*(channels!=2)` is 0 and `(channels==2)` is 0, so the whole numerator is `7`; returns `18` | `err_e2_zero_channels` | [x] |
| E3 | `max_size_frame` | `blocksize = 0` (zero "length") | no trap; numerator is `7`, returns `18 + channels` | `err_e3_zero_blocksize` | [x] |
| E4 | `max_size_frame` | `bitdepth = 0` (zero width) — note `bitdepth != 32` is still 1, so term3 is `blocksize*1*(channels==2)` and is NOT zero for stereo | no trap; stereo result is `18+2+(blocksize+7)/8` | `err_e4_zero_bitdepth` | [x] |
| E5 | `max_size_frame` | oversized: `UINT32_MAX` in each argument position individually | no trap; unsigned wraparound modulo 2^32 | `err_e5_u32_max_each` | [x] |
| E6 | `max_size_frame` | oversized: `(UINT32_MAX, UINT32_MAX, UINT32_MAX)` — maximal wraparound in every multiply and in the `+7` and `18+channels` adds | no trap; wrapping result | `err_e6_u32_max_all` | [x] |
| E7 | `max_size_frame` | numerator overflow chosen so `term1` alone wraps (`blocksize*bitdepth*channels > 2^32`) | no trap; wrapped numerator then `/8` | `err_e7_numerator_overflow` | [x] |
| E8 | `max_size_frame` | `+7` carry overflow: inputs making the pre-`+7` sum land in `[2^32-7, 2^32-1]` so the `+7` itself wraps to a tiny value | no trap; wrapped-then-divided value | `err_e8_plus7_wrap` | [x] |
| E9 | `max_size_frame` | final-add overflow: result of `18 + channels + bytes` wraps past `2^32` | no trap; wrapping sum | `err_e9_final_add_wrap` | [x] |
| E10 | `max_size_frame` | one step past the interesting range boundaries: `bitdepth` in `{31, 32, 33}` and `channels` in `{1, 2, 3}` (the only two values the C special-cases) | no trap; the `==`/`!=` flags flip exactly at 2 and 32 | `err_e10_one_past_boundaries` | [x] |
| E11 | `max_size_frame` | out-of-"enum"-range: `channels` and `bitdepth` far outside any musically meaningful domain (e.g. `channels = 0xFFFF_FFFE`, `bitdepth = 0xDEAD_BEEF`). C accepts any `uint32_t`, so these are real inputs | no trap; value follows the same unsigned formula | `err_e11_out_of_domain_values` | [x] |
| E12 | `max_size_frame` | division: numerator values `0..=15` and `2^32-1` exercise the `/8` truncation edges, including the case where wraparound makes the numerator smaller than 8 | no trap; unsigned truncating division, never a divide-by-zero (divisor is the literal 8) | `err_e12_division_edges` | [x] |

All 12 rows have a passing differential test in
`tests/differential.rs`; see the `error_paths` module.
