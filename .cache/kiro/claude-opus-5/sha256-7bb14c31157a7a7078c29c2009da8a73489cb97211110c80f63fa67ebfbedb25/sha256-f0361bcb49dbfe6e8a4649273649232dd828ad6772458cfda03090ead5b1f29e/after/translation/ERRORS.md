# ERRORS.md — Error-surface table

## How this table was derived

Mechanical grep over the whole of `c_src` (2 files, 1 translation unit) for
every rejection mechanism a C library can use:

```
grep -nE 'return|assert|RETURN_ERROR|NULL|errno|exit|abort|<|>|==|!=|if|switch|\?' \
     c_src/src/driver.c c_src/include/driver.h
```

Result: the only line that matches at all is `c_src/include/driver.h:29:
#endif //DRIVER_H_`, a false positive on the substring `if`. **Zero real
matches** for `return` with a value, `assert`, `errno`, `exit`, `abort`, any
error enum, any `if` / `switch` / ternary, any comparison operator, any null
check, and any min/max constant. The complete body is:

```c
void driver(int x, int y) {
    div_t result = div(x, y);
    printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
}
```

`driver` returns `void`, so there is no error code and no sentinel value to
compare. There are no pointer parameters, so there is no null-pointer path.
There are no enum parameters, so there is no out-of-range-enum path. Both
parameters are `int`, and **every one of the 2^64 `(int, int)` pairs is an
accepted input** as far as the source is concerned.

That means the error surface is not in the source text — it is in the two
operations the source delegates to:

1. `div()` (glibc, imported: `U div@GLIBC_2.2.5`), which performs `numer / denom`
   and `numer % denom` on `int`. These are *undefined behavior* for two operand
   pairs, and on x86-64 the `idiv` instruction signals `#DE` (divide error) for
   both, delivered to the process as **SIGFPE**. That is the library's only
   observable rejection: the process dies on signal 8 and prints nothing.
2. `printf()` — whose return value the C **discards**, so a write failure (e.g.
   fd 1 closed, `EPIPE`, ENOSPC) is *not* an error path: `driver` ignores it and
   returns normally. This is deliberate in the C and must be replicated: the
   Rust also discards `printf`'s return value. Listed as row 3 so the
   "no rejection happens here" behavior is actually tested rather than assumed.

## The table

Every row is a *distinct* rejection/termination condition. "expected C result"
is the exact observable, not "fails somehow".

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `driver` | `y == 0`, any `x` (tested with `x` = 0, 1, -1, 7, -7, `INT_MAX`, `INT_MIN`, and randomized) — `idiv` divide-by-zero | process killed by **SIGFPE (signal 8)**; **no** stdout output (the `printf` is never reached); no normal return, so no exit code |
| 2 | `driver` | `x == INT_MIN && y == -1` — quotient `2147483648` is not representable in `int`; `idiv` signed overflow | process killed by **SIGFPE (signal 8)**; **no** stdout output |
| 3 | `driver` | `printf` fails: fd 1 closed / redirected to a closed pipe, so the eventual write returns `-1` | **NOT an error**: return value discarded, `driver` returns normally, process exits 0. Nothing is observable to the caller. |
| 4 | `driver` | value one step past the boundary of row 2 on each axis: `(INT_MIN, -2)`, `(INT_MIN, 1)`, `(INT_MIN+1, -1)`, `(INT_MAX, -1)` | **NOT an error**: all four are representable, so normal return with the printed quotient/remainder. Confirms the trap in row 2 is exactly one point, not a range. |
| 5 | `driver` | out-of-range "enum" value across the FFI boundary — N/A by construction, but the closest analogue is the full `int` domain being accepted: the extreme bit patterns `INT_MIN`, `INT_MAX`, `-1`, `0` passed in either position | **NOT an error** except where rows 1–2 apply; every other bit pattern is a valid input and must print. Tested exhaustively over the 7×7 grid of extremes. |

Rows 3–5 are "negative" rows: the point of each is that the C **does not**
reject, and the Rust must therefore not reject either (a Rust translation that
returned early, panicked, or validated would diverge). They are included
because the anti-blind-spot rule is about every *distinct rejection decision*
the C makes, including the decisions to accept.

## Why rows 1–2 are the hard part of this translation

Idiomatic Rust `x / y` and `x % y` do not trap — they **panic**, which unwinds
or (with `panic = "abort"`, as this crate sets) prints
`thread '<unnamed>' panicked at ... attempt to divide by zero` to **stderr**
and raises **SIGABRT (signal 6)**. `wrapping_div` is worse for row 2: it
*returns* `INT_MIN` and prints a line where C printed nothing. Either is a
visible divergence in both the signal number and the stdout bytes. The Rust
translation therefore emits `cdq; idiv` through `core::arch::asm!` on x86-64 so
that the same `#DE` fault is raised at the same point.

Checklist (checked only when the differential test for the row passes against
both `.so`s):

- [x] 1 — `test_row1_divide_by_zero_signals_identically`
- [x] 2 — `test_row2_int_min_div_neg_one_signals_identically`
- [x] 3 — `test_row3_printf_failure_is_ignored`
- [x] 4 — `test_row4_one_step_past_the_overflow_point`
- [x] 5 — `test_row5_full_extreme_value_grid`
