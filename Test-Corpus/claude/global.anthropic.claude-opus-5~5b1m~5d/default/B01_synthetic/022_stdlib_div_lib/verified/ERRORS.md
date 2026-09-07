# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every rejection construct was grepped for across the whole C tree:

```
$ grep -nE 'return|assert|RETURN|NULL|errno|if *\(|switch|#ifdef|#if |MAX|MIN|<=|>=|exit|abort' \
      -r c_src/src c_src/include
<no matches>
```

Findings, stated precisely:

* `driver` returns `void` — there is **no error return value, no sentinel, and no
  out-parameter status**. It cannot signal failure to its caller by value.
* There are **no `assert`s**, no `RETURN_ERROR`-style macros, no error enums, no
  `return -1` / `return NULL`, no `errno` inspection.
* There are **no explicit range checks, null checks, or min/max constants**. The
  function takes two `int` by value; there are no pointers to null-check and no
  lengths to bound-check.
* `driver` has **no branches at all** (no `if`, `switch`, or `#ifdef`).

Therefore the entire rejection surface of this library is the **trap behaviour of
the two degenerate inputs to `div(3)`**, which the C standard leaves undefined
and which x86-64 realises as the `#DE` (divide error) fault delivered as
`SIGFPE`. These are genuine inputs a caller can pass, so they are real rows.

`printf` can fail (returns negative on I/O error), but `driver` discards its
return value, so a `printf` failure is not observable through this API and is
not a distinct rejection row.

## The table

One row per distinct way the C rejects / faults on input.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| E1 | `driver` | `y == 0`, `x` arbitrary (e.g. `x = 7, y = 0`) — integer division by zero inside `div` | Process terminates by **`SIGFPE` (signal 8)**. No line is printed to stdout (stdout stays empty). | [x] |
| E2 | `driver` | `y == 0`, `x == 0` — the `0/0` form of division by zero | Process terminates by **`SIGFPE` (signal 8)**; stdout empty. | [x] |
| E3 | `driver` | `y == 0`, `x == INT_MIN` (`-2147483648`) — division by zero at the extreme numerator | Process terminates by **`SIGFPE` (signal 8)**; stdout empty. | [x] |
| E4 | `driver` | `y == 0`, `x == INT_MAX` (`2147483647`) — division by zero at the other extreme | Process terminates by **`SIGFPE` (signal 8)**; stdout empty. | [x] |
| E5 | `driver` | `x == INT_MIN && y == -1` — signed-division overflow (`2147483648` is not representable in `int`) | Process terminates by **`SIGFPE` (signal 8)** on x86-64; stdout empty. | [x] |

Rows E1–E4 are kept distinct because the trigger `y == 0` interacts with the
numerator: a naive translation that guards only "obvious" `x/y` cases, or that
uses Rust's `checked_div`/`wrapping_div` (which return `None`/`INT_MIN` instead
of faulting), diverges on a *different subset* of these than one that special
cases `0/0`. Each is asserted independently.

Row E5 is the overflow trap, a different CPU condition from divide-by-zero and
the one most commonly mistranslated: Rust's `i32::wrapping_div(INT_MIN, -1)`
returns `INT_MIN` and prints a line, whereas the C **crashes**. Rust's plain `/`
operator panics in debug and is UB-checked in release, which also diverges. The
translation avoids all of these by calling libc `div` itself.

## Generic FFI boundaries also covered by the Phase C test file

Beyond the table (as required), the error-path tests additionally cover:

| boundary | how it applies here | expectation |
|----------|---------------------|-------------|
| null pointers | **N/A by construction** — the ABI is `void driver(int, int)`; there is no pointer parameter, so there is no null pointer to pass. Asserted by inspection of `driver.h`. | — |
| zero length | **N/A** — no length/count parameter exists. | — |
| oversized length | **N/A** — no length/count parameter exists. | — |
| one step past a valid range | `x`/`y` are unrestricted `int`; the representable extremes `INT_MIN` and `INT_MAX` *are* the boundary, and `INT_MIN ± 1` / `INT_MAX ± 1` wrap into the same `int` domain. Both extremes and their neighbours (`INT_MIN+1`, `INT_MAX-1`) are tested as valid inputs in `CONFIGS.md` rows C10–C13. | identical output |
| out-of-range enum values across FFI | **N/A** — no `enum`, no mode/flag parameter appears anywhere in the public header. There is no integer parameter with a restricted variant set: every one of the 2^32 bit patterns of `x` and `y` is a meaningful `int`. Covered instead by full-range randomized `i32` sampling in Phase B (row C14), which includes bit patterns no hand-written test would pick. | identical output |
| truncated/garbage arguments | An `int` argument register holding any 32-bit pattern is valid; row C14 samples uniformly from all of `i32`. | identical output |

## Result

All 5 rows have a passing differential test in
`translation/tests/error_paths.rs` (`phase_c_error_surface`), which asserts the
**same termination signal** for C and Rust — signal 8 specifically, not merely
"both failed somehow" — and that **both produced identical (empty) stdout**.
