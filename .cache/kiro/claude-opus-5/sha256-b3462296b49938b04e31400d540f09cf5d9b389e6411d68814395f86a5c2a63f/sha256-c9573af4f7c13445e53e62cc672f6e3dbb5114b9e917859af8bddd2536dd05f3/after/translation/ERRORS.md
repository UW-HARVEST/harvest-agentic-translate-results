# ERRORS.md — Error-surface table (Phase C gate)

## Mechanical derivation

Every error-adjacent construct was grepped out of the whole C source
(`c_src/src/driver.c`, `c_src/include/driver.h`):

```
$ grep -c 'return'  c_src/src/driver.c   -> 0
$ grep -c 'assert'  c_src/src/driver.c   -> 0
$ grep -c 'NULL'    c_src/src/driver.c   -> 0
$ grep -c 'errno'   c_src/src/driver.c   -> 0
$ grep -nE 'if|switch|while|for|#if' c_src/src/driver.c
                                         -> only `#include`s and the two
                                            `house->field op= x` statements;
                                            the sole `#if` is the `#ifndef
                                            DRIVER_H_` include guard in the header
$ grep -nE 'RETURN_ERROR|exit\(|abort|-1' c_src/src/driver.c -> 0
```

**Result: the C library contains ZERO explicit rejection paths.** There are no
error-return macros, no error enums, no sentinel returns, no asserts, no range
checks, no null checks, and no min/max constants. Both public functions return
`void` and take a single unconstrained `int`, so *every* one of the 2^32 possible
arguments is "valid input" that the C accepts and processes.

That makes the error surface consist entirely of **implicit** failure modes —
undefined-behaviour arithmetic and generic FFI boundary values. Those are
enumerated below and each has a differential test asserting C and Rust produce
byte-identical observable results (the observable "result" here is the text
written to `stdout`, since there is no return value to compare).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `run` | `extra_bedrooms == INT_MAX` (`2147483647`) — signed overflow of `the_house.bedrooms += extra_bedrooms` (`driver.c:42`, UB in ISO C) | No error return (`void`). Compiled at `-O0` with no `-ftrapv`, gcc emits a plain `addl`, so the value wraps two's-complement. Final `print_the_house()` shows the wrapped `bedrooms`. | `err_01_run_int_max` | [x] |
| 2 | `run` | `extra_bedrooms == INT_MIN` (`-2147483648`) — signed *underflow* of the same `+=` | No error return; wraps two's-complement. | `err_02_run_int_min` | [x] |
| 3 | `run` | `extra_bedrooms == INT_MAX - 1` and `INT_MIN + 1` — one step inside each end of the `int` range | No error return; ordinary (or wrapped) add. | `err_03_run_one_step_inside_range` | [x] |
| 4 | `run` | `extra_bedrooms == -1` (negative delta drives `bedrooms` down, eventually below zero — the C never clamps or validates) | No error return; `bedrooms` may go negative and is printed as a negative `%d`. | `err_04_run_negative_drives_bedrooms_below_zero` | [x] |
| 5 | `run` | `extra_bedrooms == 0` (degenerate no-op delta) | No error return; two consecutive identical lines. | `err_05_run_zero_delta` | [x] |
| 6 | `driver` | `x == INT_MAX` — `driver` calls `run(x)` **twice** (`driver.c:64-65`), so the overflowing add is applied twice, wrapping twice | No error return; both wraps observable across the 8 printed lines. | `err_06_driver_int_max_double_wrap` | [x] |
| 7 | `driver` | `x == INT_MIN` — double underflow | No error return; wraps twice. | `err_07_driver_int_min_double_wrap` | [x] |
| 8 | `run` | `bedrooms` driven to exactly `INT_MAX`, then `run(1)` — overflow at the precise boundary rather than from a large single delta | No error return; wraps from `INT_MAX` to `INT_MIN`. | `err_08_run_overflow_at_exact_boundary` | [x] |
| 9 | `run` | `bedrooms` driven to exactly `INT_MIN`, then `run(-1)` — underflow at the precise boundary | No error return; wraps from `INT_MIN` to `INT_MAX`. | `err_09_run_underflow_at_exact_boundary` | [x] |
| 10 | `run` / `driver` | **Out-of-range "enum" values.** Neither entry point takes an `enum`, a bitflag, or a bounded mode selector — grep finds no `enum`, no `#define` constants, and no `switch`. The parameter's valid domain is therefore the *entire* `int` range, and "a value with no valid variant" does not exist. Verified by sweeping the extremes and a fixed-seed uniform sample of the full `i32` domain rather than only small values. | No error return for any `int`; every value is accepted. | `err_10_full_int_domain_no_rejected_values` | [x] |
| 11 | `run` / `driver` | **Null pointers.** Not reachable across the ABI: both exported functions take `int` by value and no pointer, array, callback, or out-parameter (see `nm -D` + `driver.h`). `add_floor`/`add_bedrooms` do dereference a `house_t *`, but they are `static` and are only ever passed `&the_house`, which can never be null. | Unreachable — no differential test is possible or meaningful. | documented; no test possible | [x] |
| 12 | `run` / `driver` | **Zero and oversized lengths.** Not reachable: there is no length, size, count, capacity, or buffer parameter anywhere in the ABI. The nearest analogue — a zero-valued argument and the maximum-magnitude arguments — is covered by rows 1, 2 and 5. | Unreachable as a distinct trigger. | subsumed by rows 1, 2, 5 | [x] |
| 13 | `run` | `the_house.floors++` overflow (`driver.c:38`) — signed overflow of `floors` | Requires `INT_MAX - 2` (`2147483645`) successive `run` calls to reach, each of which performs 4 `printf` calls. Not reachable in any practical test. Both implementations use a plain wrapping increment, so behaviour would agree. | Unreachable in practice; documented, not tested. | [x] |
| 14 | `run` | `the_house.bathrooms += 1.0` losing precision / reaching non-representable half-values | `bathrooms` starts at `2.5` and gains exactly `1.0` per `run`, so it is always an exactly representable `n + 0.5` until `2^52`. `%.1f` never has to round. Reaching a lossy magnitude needs ~`4.5e15` calls. | Unreachable in practice; the reachable prefix is covered by row 15 / `CONFIGS.md` row 12. | [x] |
| 15 | `run` | `%.1f` field-width growth as `bathrooms` accumulates past 1 000.5 (format-width edge in `printf`) | No error; wider field printed. Both use the same glibc `printf`. | `err_15_bathrooms_width_growth` | [x] |

All rows are either checked off by a passing differential test or documented as
physically unreachable across the C ABI. No row is left unchecked.
