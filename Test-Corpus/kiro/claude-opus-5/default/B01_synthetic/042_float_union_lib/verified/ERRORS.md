# ERRORS.md — error-surface table (Phase A / gate for Phase C)

## How this table was derived

Mechanically, from `c_src/src/driver.c` and `c_src/include/driver.h`. The entire
non-comment body of the library is:

```c
#include "driver.h"
#include <stdint.h>
#include <stdio.h>

typedef union {
    uint64_t x;
    double f;
} raw_double_t;

void driver(double f) {
    raw_double_t u = {.f = f};
    printf("%llx %a %.4f\n", u.x, f, f);
}
```

Grepping the code region (lines 23..EOF, i.e. everything after the licence
comment) for every rejection construct gives all-zero counts:

| construct | `return` | `assert` | `NULL` | `errno` | `if` | `switch` | `#if` | `for` | `while` | `goto` | `exit` | `abort` | `[` (index) | `malloc` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| occurrences | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

So: **there is no error-return macro, no error enum, no sentinel return, no
`assert`, no range check, no null check, and no min/max constant in this C
library.** `driver` returns `void` and discards `printf`'s return value, so no
failure of any kind is ever reported to the caller. Rows 1–4 below record that
absence explicitly (it is a finding, not an omission), and the remaining rows are
the generic-boundary / degenerate-input cases Phase C mandates regardless: every
input class where the C's *observable behaviour* (the bytes on stdout) is
produced by an exceptional branch inside the glibc conversions the C invokes.

`expected C result` is the exact stdout line, written as
`<%llx> <%a> <%.4f>` followed by `\n`. Anything that is value-dependent is
marked and checked by the differential test rather than hard-coded.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `driver` | any input whatsoever — the function has no rejection branch | never returns an error; `void` return, no error code, no sentinel, no `errno` set by the library itself |
| 2 | `driver` | `printf` itself fails (return < 0) | return value is discarded; `driver` still returns normally. Rust must also ignore the write result and not panic |
| 3 | `driver` | NULL pointer argument | **N/A** — `driver` has no pointer parameter; nothing to null-check |
| 4 | `driver` | out-of-range enum value / zero or oversized length | **N/A** — `driver` has no enum and no length/count parameter. Its one parameter is `double`, for which *all* 2^64 bit patterns are legal arguments; there is no value the C rejects |
| 5 | `driver` | `f` = quiet NaN, sign bit clear (`0x7ff8000000000000`) | `7ff8000000000000 nan nan\n` |
| 6 | `driver` | `f` = quiet NaN, sign bit set (`0xfff8000000000000`) | `fff8000000000000 -nan -nan\n` — glibc honours the NaN sign bit for both `%a` and `%f` |
| 7 | `driver` | `f` = signalling NaN (`0x7ff4000000000000`) | `7ff4000000000000 nan nan\n` — `printf` does not trap; sNaN is spelled the same as qNaN |
| 8 | `driver` | `f` = NaN with non-canonical payload, e.g. `0x7ff0000000000001` (smallest NaN), `0xffffffffffffffff` | `%llx` prints the payload exactly; `%a`/`%.4f` print `nan` / `-nan` with no payload digits |
| 9 | `driver` | `f` = `+inf` (`0x7ff0000000000000`) | `7ff0000000000000 inf inf\n` |
| 10 | `driver` | `f` = `-inf` (`0xfff0000000000000`) | `fff0000000000000 -inf -inf\n` |
| 11 | `driver` | `f` = `+0.0` (`0x0000000000000000`) | `0 0x0p+0 0.0000\n` — `%llx` of zero is a single `0`; `%a` uses leading digit `0`, emits **no** radix point, and exponent `p+0` |
| 12 | `driver` | `f` = `-0.0` (`0x8000000000000000`) | `8000000000000000 -0x0p+0 -0.0000\n` — the sign must survive into both conversions even though the magnitude is zero |
| 13 | `driver` | `f` = smallest positive subnormal (`0x0000000000000001`) | `1 0x0.0000000000001p-1022 0.0000\n` — glibc does **not** renormalise subnormals: leading digit stays `0` and the exponent is pinned at `BIAS-1 = -1022` |
| 14 | `driver` | `f` = largest subnormal (`0x000fffffffffffff`) | `fffffffffffff 0x0.fffffffffffffp-1022 0.0000\n` |
| 15 | `driver` | `f` = negative smallest subnormal (`0x8000000000000001`) | `8000000000000001 -0x0.0000000000001p-1022 -0.0000\n` — the `%.4f` result is negative zero with a sign |
| 16 | `driver` | `f` = smallest positive **normal**, `DBL_MIN` (`0x0010000000000000`) | `10000000000000 0x1p-1022 0.0000\n` — exponent field 1 is the first value that switches the `%a` leading digit to `1`; note the same `p-1022` as row 13 but a different leading digit |
| 17 | `driver` | `f` = `DBL_MAX` (`0x7fefffffffffffff`) | `7fefffffffffffff 0x1.fffffffffffffp+1023 ` + a 309-integer-digit decimal + `.0000\n` — the largest exponent the `%a` path can emit and the longest `%.4f` output |
| 18 | `driver` | `f` = `-DBL_MAX` (`0xffefffffffffffff`) | as row 17, negated, with `-0x1.…p+1023` |
| 19 | `driver` | `f` one step past the `%.4f` "rounds to zero" boundary from below: largest double `< 0.00005` (`0.000049999999999999996`) | fraction rounds **down**: `… 0.0000\n` |
| 20 | `driver` | `f` = nearest double to the exact tie `0.00005` (`0x3f0a36e2eb1c432d`, which is `0.000050000000000000002...` — *above* the tie) | rounds **up**: `… 0.0001\n`. The tie is unrepresentable, so the exact expansion decides; the Rust must use the exact expansion too, not a shortest-repr shortcut |
| 21 | `driver` | `f` = an exactly-representable `%.4f` tie whose 4th fraction digit is **even**: `0.03125` (= 1/32, exact decimal `0.03125`, so the digit past the cut is exactly 5 and nothing follows) | round-half-to-**even** keeps the 4th digit: `0.0312`, not `0.0313` |
| 22 | `driver` | `f` = an exactly-representable `%.4f` tie whose 4th fraction digit is **odd**: `0.09375` (= 3/32, exact decimal `0.09375`) | round-half-to-even rounds **up**: `0.0938` |
| 23 | `driver` | `f` = a value whose `%.4f` rounding carries all the way, e.g. `0.99999` → `1.0000`, and `9.99999` → `10.0000` | carry propagates into the integer part; digit count grows |
| 24 | `driver` | `f` = exponent field `0x7fe` with mantissa 0 (`0x7fe0000000000000`) — one step below the inf exponent | `%a` = `0x1p+1023`; must **not** be classified as inf |
| 25 | `driver` | `f` = mantissa with trailing zero nibbles, e.g. `0x3ff1230000000000` | `%a` trims trailing hex zeroes: `0x1.123p+0`, and does not emit the full 13 digits |
| 26 | `driver` | `f` = mantissa whose low nibble is nonzero, `0x3ff0000000000001` | `%a` keeps all 13 digits: `0x1.0000000000001p+0` (no trimming, leading zeroes inside the fraction preserved) |
| 27 | `driver` | `f` = `1.0` (`0x3ff0000000000000`), exponent field exactly `BIAS` | `3ff0000000000000 0x1p+0 1.0000\n` — the `p+0` boundary between the "positive exponent" and "negative exponent" branches; sign of the exponent is `+`, never absent |
| 28 | `driver` | `f` = value just below 1, `0x3fefffffffffffff` | `%a` exponent flips to `p-1`: `0x1.fffffffffffffp-1` |
| 29 | `driver` | `f` bit pattern `0x0000000000000000`..`0x000000000000000f` (a `%llx` result of exactly one hex digit) | `%llx` prints no leading zeroes and no padding, so a 1-digit result stays 1 digit |
| 30 | `driver` | stdout closed (`close(1)`) or pointed at an unwritable fd before the call | `printf` fails with EBADF, the return value is discarded, `driver` returns normally, nothing is written. Rust must match: no panic, no abort (the crate is built with `panic = "abort"`, so a panic here would kill the process) |
| 31 | `driver` | called repeatedly / re-entrantly with the buffered `FILE*` mid-line | output is appended to the same `stdout` `FILE` buffer; no per-call flush is forced by the C. Rust must use the same `FILE` object so interleaving is identical |
| 32 | `driver` | stdout forced **wide-oriented** with `fwide(stdout, 1)` before the call | glibc's byte functions refuse a wide-oriented stream, so `printf` fails and writes nothing; the return value is discarded and `driver` returns normally. Rust's `fwrite` must fail the same way — same (empty) output, same exit status, no panic |
| 33 | `driver` | stdout switched to `_IONBF` / `_IOLBF` / `_IOFBF` with `setvbuf` before the call | the bytes are identical in all three modes; only the flush timing differs. Because both libraries go through the *same* `FILE`, the Rust must produce byte-identical output under each mode |

Rows 1–4 are "N/A / absent by construction" findings; rows 5–33 each get a
differential test in `tests/errors.rs`. Rows 2, 30, 32 and 33 run in a forked
child, because a failed write and `fwide` both leave sticky state on glibc's
`stdout` that cannot be undone — and because the crate is built with
`panic = "abort"`, a fork also makes "did the Rust panic?" observable as
`SIGABRT` rather than as a dead test process.

## Phase C checklist

| # | test | status |
|---|------|--------|
| 1 | `err_01_no_error_return` (`void` return, no observable error channel) | [x] |
| 2 | `err_02_printf_failure_ignored` | [x] |
| 3 | `err_03_null_pointer_na_but_abi_checked` (N/A; the substitutable check is the calling convention) | [x] |
| 4 | `err_04_enum_and_length_na_no_value_is_rejected` (N/A; 200k-pattern fuzz stands in) | [x] |
| 5 | `err_05_qnan_positive` | [x] |
| 6 | `err_06_qnan_negative` | [x] |
| 7 | `err_07_snan` | [x] |
| 8 | `err_08_nan_noncanonical_payloads` | [x] |
| 9 | `err_09_pos_inf` | [x] |
| 10 | `err_10_neg_inf` | [x] |
| 11 | `err_11_pos_zero` | [x] |
| 12 | `err_12_neg_zero` | [x] |
| 13 | `err_13_min_subnormal` | [x] |
| 14 | `err_14_max_subnormal` | [x] |
| 15 | `err_15_neg_min_subnormal` | [x] |
| 16 | `err_16_dbl_min_normal` | [x] |
| 17 | `err_17_dbl_max` | [x] |
| 18 | `err_18_neg_dbl_max` | [x] |
| 19 | `err_19_just_below_rounding_boundary` | [x] |
| 20 | `err_20_nearest_double_to_tie` | [x] |
| 21 | `err_21_exact_tie_even` | [x] |
| 22 | `err_22_exact_tie_odd` | [x] |
| 23 | `err_23_rounding_carry` | [x] |
| 24 | `err_24_max_exponent_field_not_inf` | [x] |
| 25 | `err_25_trailing_zero_trim` | [x] |
| 26 | `err_26_low_nibble_set_no_trim` | [x] |
| 27 | `err_27_exponent_zero_boundary` | [x] |
| 28 | `err_28_exponent_minus_one_boundary` | [x] |
| 29 | `err_29_single_hex_digit_llx` | [x] |
| 30 | `err_30_stdout_closed` | [x] |
| 31 | `err_31_repeated_calls_share_buffer` | [x] |
| 32 | `err_32_wide_oriented_stream` | [x] |
| 33 | `err_33_buffering_modes` | [x] |

All 33 rows pass against BOTH the debug and the release `cdylib`
(`cargo test --test errors` → 33 passed, 0 failed; the two `soak_*` tests in the
same file are `#[ignore]`d and were run separately, see `CONFIGS.md`).
