# ERRORS.md — Error-surface table (Phase A → gate for Phase C)

Derived mechanically from the C sources, not from docs or assumptions.

## Mechanical grep of the C sources

```sh
grep -nE "return|assert|NULL|errno|if|switch|#if|exit|abort|-1|MAX|MIN" \
     c_src/src/driver.c c_src/include/driver.h
```

Non-comment hits:

```
c_src/src/driver.c:26:#include <stdio.h>
c_src/src/driver.c:27:#include <stdlib.h>
c_src/include/driver.h:24:#ifndef DRIVER_H_
c_src/include/driver.h:29:#endif //DRIVER_H_
```

That is the complete result: the only two non-comment statements in the library
are

```c
div_t result = div(x, y);
printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
```

So the C library contains:

* **no** `return` statements (the only function is `void driver(int, int)`)
* **no** error-return macros, **no** error enums, **no** sentinel values
* **no** `assert`
* **no** explicit range checks, **no** null checks
* **no** min/max constants
* **no** `errno` inspection; `printf`'s return value is discarded

There is therefore **no software-level rejection path at all**. The complete
rejection surface is the *hardware* trap raised inside glibc's `div(3)`, which
`driver` invokes with entirely unvalidated arguments. Both rows below are real,
reachable inputs that an external caller can pass through the FFI boundary, and
the Rust must fail identically.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `driver` | `y == 0` (any `x`) — `div(x, 0)` executes `idiv` with a zero divisor, raising `#DE` | process dies from **`SIGFPE` (8)**; no line of output produced; `driver` never returns |
| 2 | `driver` | `x == INT_MIN (-2147483648) && y == -1` — quotient `2147483648` is not representable in 32 bits, so `idiv` raises `#DE` (overflow) | process dies from **`SIGFPE` (8)**; no line of output produced; `driver` never returns |

Row 1 is the `y == 0` case for *every* `x`, including `x == 0`; the tests sample
many `x` values (including `0`, `INT_MIN`, `INT_MAX`) against `y == 0`.

## Generic FFI-boundary boundaries also covered by Phase C

`driver` takes two by-value `int`s and returns `void`, so several generic
categories are *structurally* inapplicable — recorded here so the omission is
deliberate rather than an oversight:

| generic boundary | applicability to `void driver(int, int)` | covered by |
|---|---|---|
| null pointers | N/A — no pointer parameters, no pointer return | — |
| zero / oversized lengths | N/A — no length or buffer parameters | — |
| out-of-range enum values | N/A — no enum parameters | — |
| out-of-range integer values | every 32-bit pattern is a valid `int`; there is no documented valid range to step past | `test_extremes_and_one_step_past_boundaries`, `test_full_int_boundary_matrix` |
| one step past a boundary | `INT_MIN`/`INT_MAX` cannot be stepped past in `int`; instead all of `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` are crossed with each other (49 pairs) | `test_full_int_boundary_matrix` |
| trap-adjacent values | `y == 0` and `INT_MIN / -1` are the two traps; their neighbours (`y == ±1`, `x == INT_MIN` with `y == 1`, `INT_MIN+1 / -1`) must *not* trap and must match | `test_trap_neighbours_do_not_trap` |

## Row check-off (Phase C)

| # | trigger | differential test | result |
|---|---------|-------------------|--------|
| 1 | `y == 0` | `test_err_row1_divide_by_zero_sigfpe` | [x] both C and Rust die with `SIGFPE` (8), no output |
| 2 | `INT_MIN / -1` | `test_err_row2_int_min_div_neg_one_sigfpe` | [x] both C and Rust die with `SIGFPE` (8), no output |

Because these rows kill the calling process, each test re-`exec`s the test binary
as a worker child (one per side), runs the call there with `stdout` redirected to
a private file, and compares `WIFSIGNALED`/`WTERMSIG`/`WEXITSTATUS` plus the
bytes written before the trap. "Same rejection" is asserted as the *same signal
number*, not merely "both died".

## Phase C results

`tests/differential_errors.rs` — **12 tests, all passing** in both the dev and
release profiles.

| test | covers | inputs |
|---|---|---|
| `test_err_row1_divide_by_zero_sigfpe` | **row 1** | 18 dividends (13 fixed incl. `0`, `INT_MIN`, `INT_MAX` + 5 randomized) × `y == 0` |
| `test_err_row1_output_before_trap_matches` | **row 1** | 3 good calls then `driver(123, 0)`; both sides must emit the same 3 lines and then die |
| `test_err_row2_int_min_div_neg_one_sigfpe` | **row 2** | `driver(INT_MIN, -1)`, plus 5 near-miss pairs that must *not* trap |
| `test_err_row2_traps_with_garbage_upper_halves` | **rows 1 & 2** | both traps reached through the `fn(i64, i64)` re-typing, with non-zero upper register halves |
| `test_err_harness_distinguishes_trap_from_success` | harness control | proves a clean run and a trap are distinguishable, so rows 1–2 cannot pass vacuously |
| `test_trap_neighbours_do_not_trap` | generic | 16 pairs adjacent to both traps; asserts *no* signal on either side and identical output |
| `test_full_int_boundary_matrix` | generic | 11 × 11 = 121 pairs: 96 compared, 25 trap-compared |
| `test_extremes_and_one_step_past_boundaries` | generic | 14 × 14 extremes cross-product (traps excluded) |
| `test_garbage_in_upper_register_halves` | generic FFI | 500 randomized calls with non-zero garbage in the upper halves of RDI/RSI, cross-checked against the clean 32-bit calls |
| `test_void_return_and_no_pointer_parameters` | generic FFI | 6 pairs; documents that there is no pointer/length/enum parameter to abuse |
| `test_no_extra_or_missing_public_symbol` | symbol surface | `driver` resolves in both `.so`s at *different* addresses; 4 plausible-but-absent names must be absent from both |
| `difftest_worker` | infrastructure | the child-side entry point (inert in a normal run) |

### Why "same rejection" is asserted strictly

`assert_same_trap` requires all four of:

1. the same `WTERMSIG` — `Some(8)` / `SIGFPE` on both sides (not just "died"),
2. the same `WEXITSTATUS` (`None` for both, since neither exits normally),
3. byte-identical output produced before the trap,
4. an explicit assertion that the signal *is* `SIGFPE`, so a Rust `panic` →
   `SIGABRT` (6) or a clean `exit(101)` can never be mistaken for a match.

### Mutation check

Deliberately replacing the inline `cdq; idiv` with Rust's
`wrapping_div`/`wrapping_rem` (which yields identical *values* for all
well-defined inputs but silently returns instead of trapping) is caught by
`test_err_row1_divide_by_zero_sigfpe`, `test_err_row2_int_min_div_neg_one_sigfpe`,
`test_err_row1_output_before_trap_matches`,
`test_err_row2_traps_with_garbage_upper_halves`, `test_full_int_boundary_matrix`
and `row17_full_boundary_matrix`. Both rows above are therefore live, not
vacuous. `src/lib.rs` was restored byte-identically after the experiment.
