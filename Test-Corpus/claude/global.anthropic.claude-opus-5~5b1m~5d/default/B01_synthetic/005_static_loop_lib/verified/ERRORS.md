# ERRORS.md — Error-surface table (Phase A, gates Phase C)

Derived mechanically from the C source. Grep performed over the whole of
`c_src/` for every rejection/error idiom:

```sh
grep -nE 'RETURN_ERROR|return *-1|return *NULL|assert|errno|exit\(|abort\(|\
          [A-Z_]*_(ERR|ERROR|MAX|MIN)|if *\(|switch *\(|goto' c_src/src/*.c c_src/include/*.h
```

Result of that grep over `src/staticloop.c` + `include/staticloop.h`:

- `return sum;` (line 31) — the ordinary success return of `static_sum`.
- `return;` (line 42) — the ordinary `void` return of `driver`.
- `#ifndef STATICLOOP_H_` / `#endif` (header include guard) — not a runtime branch.
- the loop condition `i < 10` (line 39) — a fixed loop bound, not an input check.

**There are no error returns, no error enums, no sentinel values, no `assert`s,
no `errno` use, no `exit`/`abort`, no explicit range checks, no null checks, no
min/max constants, and no pointer parameters anywhere in the C library.** Both
public functions accept every possible `int` and are total: `static_sum` always
returns a value, `driver` always completes 10 iterations.

Consequently the error surface consists solely of the *generic* C-API
boundaries that still exist for scalar-only, total functions: the extremes of
the `int` domain and the signed-overflow behaviour they provoke. Those are
enumerated below and every row has a differential test. There is no row that
can be "unchecked" for lack of an error code, because the C returns no error
codes — each row instead asserts that **C and Rust agree bit-for-bit on the
value/output produced for that boundary input**, which is the observable
"rejection or not" behaviour of this API.

| # | function | trigger (the exact invalid/extreme input or condition) | expected C result | test |
|---|----------|--------------------------------------------------------|-------------------|------|
| 1 | `static_sum` | `update == INT_MAX` on a fresh (`sum == 0`) library instance | returns `INT_MAX`; internal `sum` becomes `INT_MAX`. No error, no trap. | `err01_static_sum_int_max_fresh` |
| 2 | `static_sum` | `update == INT_MIN` on a fresh library instance | returns `INT_MIN`; internal `sum` becomes `INT_MIN`. No error, no trap. | `err02_static_sum_int_min_fresh` |
| 3 | `static_sum` | positive signed overflow: `static_sum(INT_MAX)` then `static_sum(1)` | second call wraps to `INT_MIN` (two's-complement wrap-around; C signed overflow is UB but the shipped build wraps). Returns a value, never errors. | `err03_static_sum_positive_overflow` |
| 4 | `static_sum` | negative signed overflow: `static_sum(INT_MIN)` then `static_sum(-1)` | second call wraps to `INT_MAX`. | `err04_static_sum_negative_overflow` |
| 5 | `static_sum` | `update == 0` (zero-magnitude "empty" update) | returns the unchanged accumulated `sum`; idempotent. | `err05_static_sum_zero_is_identity` |
| 6 | `static_sum` | `update == INT_MAX` applied repeatedly (many consecutive overflows) | keeps wrapping; C and Rust must agree on every intermediate return. | `err06_static_sum_repeated_int_max` |
| 7 | `static_sum` | one step past the extremes from an already-extreme state: `INT_MAX-1` then `2`, and `INT_MIN+1` then `-2` | wraps; agreement required on both returns. | `err07_static_sum_one_past_range` |
| 8 | `driver` | `stride == INT_MAX` → the multiplication `i * stride` overflows for every `i >= 2` | prints 10 lines of wrapped sums; returns void, never errors. | `err08_driver_stride_int_max` |
| 9 | `driver` | `stride == INT_MIN` → `i * stride` overflows for every `i >= 2` | prints 10 lines of wrapped sums. | `err09_driver_stride_int_min` |
| 10 | `driver` | `stride == 0` (degenerate/no-op stride) | prints the current `sum` 10 times, unchanged. | `err10_driver_stride_zero` |
| 11 | `driver` | `stride == -1` and `stride == 1` (one step either side of the degenerate 0) | prints 10 lines each; exact bytes must match. | `err11_driver_stride_pm_one` |
| 12 | `driver` | `stride` large enough that the *accumulated sum* overflows mid-loop (e.g. `stride == INT_MAX/8`, `0x2000_0000`) | some of the 10 printed values are wrapped/negative; bytes must match. | `err12_driver_sum_overflows_midloop` |
| 13 | `driver` | called on a library instance whose `sum` is already at `INT_MAX` / `INT_MIN` (extreme pre-existing state) | every iteration wraps; bytes must match. | `err13_driver_from_extreme_state` |
| 14 | both | out-of-range "enum" value across the FFI boundary | **N/A — no enum, pointer, length, or buffer parameter exists** in either signature (`int` → `int`, `int` → `void`). Every one of the 2^32 `int` bit patterns is a valid, in-range argument, so there is no representable out-of-range value to pass. Covered instead by row 15, which sweeps the full domain by random sampling plus all 32 power-of-two/complement bit patterns. | `err14_no_out_of_range_representation` (documents + asserts the exhaustive-bit-pattern sweep) |
| 15 | both | full-domain adversarial sweep: every `±2^k`, `±(2^k-1)`, `0`, `INT_MIN`, `INT_MAX`, plus 5000 seeded random `i32`s, fed to `static_sum` in sequence and to `driver` as strides | C and Rust must agree on every return value and every printed byte | `err15_full_int_domain_sweep` |

Null-pointer, zero-length and oversized-length boundaries are **structurally
inapplicable**: neither function takes a pointer or a length. This is recorded
explicitly rather than omitted, and asserted by the signature check in
`err14_no_out_of_range_representation`.
