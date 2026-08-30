# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/staticloop.c` (43 lines, 22 of which are
the licence header) and `c_src/include/staticloop.h`.

## Mechanical grep results

| pattern searched | hits in C source |
|---|---|
| `RETURN_ERROR` / `*_ERROR*` macro | 0 |
| `return -1` / negative sentinel return | 0 |
| `return NULL` / `return 0` as failure sentinel | 0 |
| `assert(` / `NDEBUG` | 0 |
| `errno` | 0 |
| explicit range check (`if (x < ... )`, `if (x > ...)`) | 0 |
| null-pointer check (`if (p == NULL)`, `if (!p)`) | 0 |
| `min`/`max`/`LIMIT`/`_MAX`/`_MIN` constants | 0 |
| error `enum` / status type | 0 |
| pointer parameters in the public API | 0 |
| allocation (`malloc`/`calloc`/`realloc`/`free`) | 0 |
| `goto` error label | 0 |

**The library has no explicit error surface.** Both public functions take a
single `int` by value, cannot fail, and return either the running sum or
nothing. `static_sum` returns `int` where *every* `int` value is a legitimate
successful result — there is no reserved sentinel — so "the same error code" for
this library means "the same `int`, bit for bit".

That is not a licence to skip Phase C. The rejection/boundary surface that
*does* exist is implicit: it lives in the arithmetic. Signed overflow is UB in
C, so the only way to know what the C `.so` actually produces is to *ask the
compiled C `.so`* and require the Rust `.so` to agree bit for bit. Each row
below is a distinct implicit boundary condition, and each has a differential
test in `tests/phase_c_errors.rs`.

## Error / boundary surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | differential test | [x] |
|---|----------|---------------------------------------------|-------------------|-------------------|-----|
| 1 | `static_sum` | `update == INT_MAX` on a fresh (`sum == 0`) library | returns `INT_MAX`; no trap, no diagnostic | `err01_sum_int_max_fresh` | [x] |
| 2 | `static_sum` | `update == INT_MIN` on a fresh library | returns `INT_MIN`; no trap | `err02_sum_int_min_fresh` | [x] |
| 3 | `static_sum` | positive overflow of `sum += update`: `sum == INT_MAX` then `update == 1` | two's-complement wrap to `INT_MIN` (UB in C; the built `.so` wraps) | `err03_positive_overflow_wraps_to_int_min` | [x] |
| 4 | `static_sum` | negative overflow of `sum += update`: `sum == INT_MIN` then `update == -1` | two's-complement wrap to `INT_MAX` | `err04_negative_overflow_wraps_to_int_max` | [x] |
| 5 | `static_sum` | maximal overflow: `sum == INT_MAX` then `update == INT_MAX` | wraps to `-2` | `err05_int_max_plus_int_max` | [x] |
| 6 | `static_sum` | `sum == INT_MIN` then `update == INT_MIN` | wraps to `0` | `err06_int_min_plus_int_min` | [x] |
| 7 | `static_sum` | `update == 0` (identity / no-op update) | returns `sum` unchanged | `err07_zero_update_is_identity` | [x] |
| 8 | `static_sum` | repeated calls that drive `sum` across the `0` boundary from above (`sum` positive, large negative `update`) | plain two's-complement result | `err08_cross_zero_boundary_values` | [x] |
| 9 | `static_sum` | truncation of an out-of-`int`-range argument passed across FFI (e.g. caller supplies `0x1_0000_0000 + 5` in a 64-bit register) | only the low 32 bits are read; behaves as `update == 5` | `err09_out_of_range_argument_truncation` | [x] |
| 10 | `driver` | `stride == INT_MAX` — `i * stride` overflows for every `i >= 2` | each iteration prints the wrapped `%d` value; 10 lines, no trap | `err10_driver_stride_int_max_product_overflow` | [x] |
| 11 | `driver` | `stride == INT_MIN` — `i * stride` overflows for every `i >= 2` | 10 lines of wrapped values, no trap | `err11_driver_stride_int_min_product_overflow` | [x] |
| 12 | `driver` | `stride == 0` — every `update` is `0` | prints the current `sum` 10 times, unchanged | `err12_driver_stride_zero_is_identity` | [x] |
| 13 | `driver` | `stride` large enough that the *accumulated* `sum` overflows even though individual `i * stride` do not (e.g. `stride == INT_MAX / 9`; note `/8` would itself overflow at `i == 9`) | wrapped running totals | `err13_driver_accumulator_only_overflow` | [x] |
| 14 | `driver` | `stride == -1` on a fresh library (drives `sum` negative monotonically) | prints `0 -1 -3 -6 -10 -15 -21 -28 -36 -45` | `err14_driver_stride_minus_one_exact_bytes` | [x] |
| 15 | `driver` | out-of-`int`-range argument across FFI (high 32 bits set) | low 32 bits used as `stride` | `err15_driver_wide_argument_truncation` | [x] |
| 16 | both | `driver` interleaved with `static_sum` — shared `static` state must be mutated by *both* entry points | single shared accumulator; `static_sum` observes `driver`'s writes and vice versa | `err16_shared_static_state_both_directions` | [x] |
| 17 | both | no reserved sentinel exists, so a "failed" call is indistinguishable from a successful one; the contract is total | never returns an error; must never abort/panic for ANY 32-bit input | `err17_total_contract_no_rejection` | [x] |

## Notes on things deliberately NOT in the table

* **Null pointers** — impossible to test: neither public function takes a
  pointer. Passing a garbage pointer-sized value is covered by rows 9 and 15
  (argument truncation).
* **Zero / oversized lengths** — the API has no length or buffer parameters.
* **Out-of-range enum values** — the API declares no enums. The nearest
  analogue is "an `int` argument with no distinguished meaning", i.e. rows 1–2
  and 10–11, plus the randomized full-`i32`-range sweep in
  `tests/phase_c_errors.rs`, which passes values with no special significance
  whatsoever.
* **Thread-safety / data races** — the C `static int sum` is unsynchronised, so
  concurrent calls are UB in C and the Rust mirrors that exactly (`UnsafeCell`
  + `unsafe impl Sync`, no locking). Not a differential-testable property.

All 17 rows are checked off in the Phase C section of the test report.
