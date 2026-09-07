# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every non-comment line of the entire C library (`c_src/src/driver.c`, after
resolving digraphs `%:`→`#`, `<%`→`{`, `%>`→`}`):

```c
#include "driver.h"
#include <stdio.h>
#include <iso646.h>

void driver(int x, int y) {
    int result = x | ~y;   /* x bitor compl y */
    printf("%d", result);
    puts("");
}
```

Greps run over `c_src/` for every rejection mechanism:

| searched for | matches in C source |
|---|---|
| `return` (any) | 0 |
| `return -1` / `return NULL` / `RETURN_ERROR` / error macros | 0 |
| `assert` | 0 |
| `errno` | 0 |
| `exit(` / `abort` | 0 |
| `if` / `switch` / `goto` / loops | 0 |
| relational / range checks (`<`, `>`, `<=`, `>=`) | 0 |
| null-pointer checks | 0 (the API takes no pointers) |
| min/max constants (`INT_MAX`, `LIMIT`, `MAX_`, `MIN_`) | 0 |
| error enums / status types | 0 (`driver` returns `void`) |

**Result: the C library has an EMPTY intrinsic error surface.** `driver` is
`void`, takes two by-value `int`s, performs a total (never-trapping) bitwise
expression, and unconditionally prints. There is no input it rejects and no
channel (return value, out-param, errno, enum) through which it could report
one. Consequently the rows below are the *generic FFI boundaries every C API
has*, as required by Phase C: extremes, one-step-past-range values, degenerate
values, and out-of-range "enum-like" integers. For each, the correct C
behaviour is **no rejection** — and the differential test asserts that the Rust
also does not reject, i.e. both print the identical bytes and both return
normally (no panic, no abort, no trap).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `driver` | `x = 0, y = 0` — degenerate all-zero input | no error; `~0 = -1`, `0 \| -1 = -1`; prints `-1\n`; returns void |
| 2 | `driver` | `x = 0, y = -1` — the only pair whose result is `0` (falsy/sentinel-looking value) | no error; prints `0\n` |
| 3 | `driver` | `x = INT_MIN (-2147483648)`, `y = 0` — extreme low `x`, sign bit set | no error; prints `-1\n` |
| 4 | `driver` | `x = INT_MAX (2147483647)`, `y = 0` — extreme high `x` | no error; prints `-1\n` |
| 5 | `driver` | `y = INT_MIN` — `~INT_MIN = INT_MAX`, complement of the extreme low value | no error; prints `x \| 0x7FFFFFFF` |
| 6 | `driver` | `y = INT_MAX` — `~INT_MAX = INT_MIN`, complement produces the sign bit | no error; prints `x \| INT_MIN` (always negative) |
| 7 | `driver` | `x = 0, y = INT_MAX` — result is exactly `INT_MIN`, a magnitude with no positive `int` counterpart (the classic negation-overflow trap for `%d` formatting) | no error; prints `-2147483648\n` (11 bytes + newline) |
| 8 | `driver` | `x = INT_MIN, y = INT_MIN` — both operands extreme low | no error; `INT_MIN \| INT_MAX = -1`; prints `-1\n` |
| 9 | `driver` | `x = INT_MAX, y = INT_MAX` — both operands extreme high | no error; `INT_MAX \| INT_MIN = -1`; prints `-1\n` |
| 10 | `driver` | one step past the top of the signed range: caller passes `0x80000000` as an unsigned 32-bit bit pattern (wraps to `INT_MIN` across the FFI boundary) | no error; identical to row 3's `x`; prints per `x \| ~y` |
| 11 | `driver` | one step past the bottom: caller passes `0x7FFFFFFF + 1` computed by wrapping arithmetic | no error; same as row 10 |
| 12 | `driver` | out-of-range "enum-like" integers: values that would have no valid variant if the parameters were enums — `x`/`y` ∈ {`-2147483648`, `-1000000`, `-2`, `3`, `42`, `255`, `256`, `65536`, `2147483647`} in all 81 combinations. C enums accept any `int`, so these are real inputs. | no error for any combination; each prints `x \| ~y` then newline |
| 13 | `driver` | sign-bit-only operands: `x = 0x80000000`, `y = 0x80000000` (single high bit) | no error; prints `-1\n` |
| 14 | `driver` | single-bit sweep: `x = 1<<i`, `y = 1<<j` for all `i, j` in `0..32` (includes `1<<31` = `INT_MIN`) — 1024 combinations, every bit position including the sign bit | no error for any pair; prints `x \| ~y` |
| 15 | `driver` | repeated invocation without intervening flush (state/buffer reuse across many back-to-back calls) | no error; outputs concatenate in call order, one line each |
| 16 | `driver` | *no* null-pointer row is possible — `driver` accepts no pointer arguments (verified: 0 `*` parameters in `driver.h`); the nearest analogue is passing the null bit pattern `0` for both scalars, i.e. row 1 | n/a |
| 17 | `driver` | *no* length/size row is possible — `driver` accepts no buffer or length argument, so "zero length" and "oversized length" are inexpressible; the nearest analogues are rows 1–2 (zero) and rows 3–4 (extreme magnitude) | n/a |

Rows 16 and 17 are recorded explicitly so the absence of null-pointer and
length checks is documented as a *derived fact about the C API's signature*,
not as an untested gap.

## Verification status

Every row has a passing differential test in `tests/phase_c_errors.rs`, each
run individually and confirmed green (`cargo test --test phase_c_errors errNN_`):

| row | test | status |
|-----|------|--------|
| 1 | `err01_both_zero` | [x] |
| 2 | `err02_result_zero_sentinel` | [x] |
| 3 | `err03_x_int_min` | [x] |
| 4 | `err04_x_int_max` | [x] |
| 5 | `err05_y_int_min` | [x] |
| 6 | `err06_y_int_max` | [x] |
| 7 | `err07_result_int_min_unnegatable` | [x] |
| 8 | `err08_both_int_min` | [x] |
| 9 | `err09_both_int_max` | [x] |
| 10 | `err10_one_past_int_max_bit_pattern` | [x] |
| 11 | `err11_wrapped_out_of_range` | [x] |
| 12 | `err12_out_of_range_enum_like_values` | [x] |
| 13 | `err13_sign_bit_only` | [x] |
| 14 | `err14_single_bit_sweep_no_rejection` | [x] |
| 15 | `err15_repeated_invocation_state` | [x] |
| 16 | `err16_no_pointer_parameters_exist` | [x] |
| 17 | `err17_no_length_parameters_exist` | [x] |

Because the C exposes no error channel, "same error/rejection" is asserted as
"same non-rejection": identical stdout bytes plus a normal return from both
libraries. Rows 16 and 17 additionally assert, from `driver.h` and `driver.c`
themselves, that no pointer/length parameter and no check statement exists, so
these tests fail loudly if the C ever grows a real error path.
