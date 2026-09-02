# ERRORS.md — Error-surface table

Mechanically derived from the **whole** C source. The library is one file,
`c_src/src/driver.c`, whose only non-comment content is:

```c
#include "driver.h"
#include <stdio.h>
#include <stdlib.h>

void driver(int x, int y) {
    div_t result = div(x, y);
    printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
}
```

Grep results over `c_src/` for every rejection construct:

| construct grepped | hits |
|---|---|
| `RETURN_ERROR` / `*_ERROR*` macro | 0 |
| `return -1` / `return 0` / any `return <value>` | 0 (function is `void`, has no `return` at all) |
| `return NULL` | 0 |
| error `enum` / status codes | 0 |
| `assert` | 0 |
| explicit range check (`if (... < ...)`, `if (... > ...)`) | 0 |
| null-pointer check | 0 (no pointer parameters exist) |
| min/max constant (`INT_MAX`, `*_MIN`, `LIMIT`, `MAX_`) | 0 |
| `errno` inspection | 0 |
| `#ifdef` / conditional compilation in the body | 0 (only the `DRIVER_H_` include guard) |

So `driver` has **no in-band error channel whatsoever**: it returns `void`,
takes two by-value `int`s, validates nothing, and the `printf` return value is
discarded. There is no input value it *rejects*.

It does, however, have three distinct **fatal rejection** conditions, and they
are real: `div()` in glibc is
`div_t result; result.quot = numer / denom; result.rem = numer % denom;`, which
on x86-64 lowers to a single `idiv` instruction. `idiv` raises `#DE`
(divide error) — delivered as `SIGFPE`, signal 8 — for both of its
non-representable operand classes. Those are the rows below. Each is a distinct
hardware/UB trigger, so each gets its own row.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `driver` | `y == 0`, `x != 0` (e.g. `x=5, y=0`) — division by zero | process terminated by `SIGFPE` (8); wait status `128+8 = 136`; **no** bytes written to stdout (fault precedes the `printf`) |
| 2 | `driver` | `y == 0`, `x == 0` (`0/0`) — indeterminate form, a *separate* operand class from row 1 | process terminated by `SIGFPE` (8); no stdout output |
| 3 | `driver` | `x == INT_MIN (-2147483648)` and `y == -1` — quotient `2147483648` is not representable in `int`; `idiv` overflow | process terminated by `SIGFPE` (8); no stdout output |

## Generic C-API boundaries (checked even though absent from the table above)

The task asks for the generic boundary classes every C API has. For this API
most are **not applicable**, and that non-applicability is itself derived from
the signature `void driver(int, int)`:

| generic boundary class | applicability here | how it is covered |
|---|---|---|
| null pointers | N/A — no pointer parameters, no pointer return | nothing to test |
| zero length / oversized length | N/A — no buffer or length parameters | nothing to test |
| out-of-range enum value across FFI | N/A — no `enum` parameter. The nearest analogue is that `int` accepts *any* 32-bit value with no "valid variant" restriction, so the entire `2^32 × 2^32` domain is legal input. Covered by treating both parameters as full-range random `i32` in `CONFIGS.md` rows 14–16, plus every extremal value in rows 11–13, 17. | `tests/difftest.rs` rows 11–17 |
| one step past a documented valid range | The header documents no range. The only *behavioural* range edges are the three fatal triggers above and the `INT_MIN`/`INT_MAX` extremes; "one step past" them is tested as `INT_MIN+1`, `INT_MAX-1`, `y = ±1`, `y = ±2` around each fatal case. | rows 1–3 here + `CONFIGS.md` rows 12, 17 |
| return-value / errno contract | N/A — `void` return, `errno` never read or set by `driver` | nothing to test |

## Checklist

- [x] Row 1 — `y == 0, x != 0` → both `.so`s die with `SIGFPE`, both produce empty stdout
- [x] Row 2 — `y == 0, x == 0` → both `.so`s die with `SIGFPE`, both produce empty stdout
- [x] Row 3 — `x == INT_MIN, y == -1` → both `.so`s die with `SIGFPE`, both produce empty stdout

Each row asserts the *same specific* termination signal (8 / `SIGFPE`) from
both libraries, not merely "both failed somehow", and additionally asserts the
stdout byte streams are identical (both empty). Tests:
`error_row_1_div_by_zero_nonzero_numer`, `error_row_2_zero_over_zero`,
`error_row_3_int_min_over_minus_one` in `tests/difftest.rs`.
