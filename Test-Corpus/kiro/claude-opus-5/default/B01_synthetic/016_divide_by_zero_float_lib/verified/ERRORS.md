# ERRORS.md — error / rejection surface table (Phase C gate)

Mechanically derived from `c_src/src/driver.c`. Grep results used:

```
grep -n 'return\|assert\|NULL\|if\s*(\|else' c_src/src/driver.c
```

Findings: the library has **no** return codes, **no** error enums, **no**
`assert`, **no** `RETURN_ERROR`-style macro, and every function returns `void`.
Its entire rejection surface consists of:

* one **null-pointer guard** (`printLine`: `if (line != NULL)`),
* one **range/magnitude guard** (`goodB2G`: `if (fabs(data) > 0.000001)`),
* the **unguarded division** in `bad()` (CWE-369 divide-by-zero — the flaw this
  test case exists to demonstrate), whose "error" is observable only as the
  value printed, because `(int)` of a non-representable `double` is UB that on
  x86-64 lowers to `cvttsd2si` and yields the *integer indefinite* value
  `INT_MIN` = `-2147483648`.

Constants that define the surface: `0.000001` (guard threshold), `100.0`
(numerator, a `double` literal, so all division happens in `double`), `2.0F`
(hard-coded safe divisor in `goodG2B`), and the implicit `INT_MIN`/`INT_MAX`
bounds of the `(int)` cast.

Confirmed against the compiled C (`objdump -d libdriver.so`):
`bad` → `cvtss2sd` / `divsd` / **`cvttsd2si`**; `goodB2G` → `andps` (inlined
`fabsf`) / `cvtss2sd` / `comisd` / **`jbe`** (so the *unordered* NaN case takes
the reject branch).

## Rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | guard fails; **no output at all**, no crash, returns normally |
| 2 | `printLine` | `line` -> `""` (valid but degenerate: empty string) | prints just `"\n"` |
| 3 | `goodB2G` (via `good`, via `driver`) | `data == 0.0f` | `fabs(0)=0`, not `> 1e-6`; prints `This would result in a divide by zero` |
| 4 | `goodB2G` (via `good`, via `driver`) | `data == -0.0f` | `andps` clears sign -> `0.0`, not `> 1e-6`; prints `This would result in a divide by zero` |
| 5 | `goodB2G` (via `good`, via `driver`) | `0 < fabs(data) <= 1e-6` (e.g. `5e-07f`, `1e-30f`, `FLT_MIN`, a subnormal such as `1e-45f`) — division would be *finite* but is still rejected | prints `This would result in a divide by zero` (NOT a number) |
| 6 | `goodB2G` (via `good`, via `driver`) | `data == 1e-6f` exactly (the boundary; `(double)1e-6f = 9.99999997e-07 < 1e-6`) | `comisd` -> below -> `jbe` taken; prints `This would result in a divide by zero` |
| 7 | `goodB2G` (via `good`, via `driver`) | `data == NaN` (quiet or signalling) — `comisd` unordered sets CF=ZF=1 so `jbe` is taken | prints `This would result in a divide by zero` |
| 8 | `goodB2G` (via `good`, via `driver`) | `data` one step *past* the guard on the accept side: smallest float `> 1e-6` (`nextafterf(1e-6f, 1f)` = `1.00000011e-06f`) | guard passes; prints `(int)(100.0/data)` = `99999989` |
| 9 | `bad` (via `driver`) | `data == 0.0f` — **no guard at all**; `100.0/0.0 = +inf`; `(int)+inf` is UB | `cvttsd2si` -> integer indefinite; prints `-2147483648` |
| 10 | `bad` (via `driver`) | `data == -0.0f`; `100.0/-0.0 = -inf` | `cvttsd2si` -> integer indefinite; prints `-2147483648` |
| 11 | `bad` (via `driver`) | `data == NaN`; `100.0/NaN = NaN`; `(int)NaN` is UB | `cvttsd2si` -> integer indefinite; prints `-2147483648` |
| 12 | `bad` (via `driver`) | `0 < fabs(data)` small enough that `100.0/data >= 2^31` (overflow past `INT_MAX`), e.g. `1e-30f`, `FLT_MIN`, subnormals | prints `-2147483648` (integer indefinite, *not* `2147483647`) |
| 13 | `bad` (via `driver`) | negative counterpart of #12: `100.0/data <= -(2^31 + 1)` (underflow past `INT_MIN`), e.g. `-1e-30f` | prints `-2147483648` |
| 14 | `bad` (via `driver`) | `data == +inf` / `-inf`; `100.0/inf = 0.0` (in range, no UB) | prints `0` — must NOT be treated as an error |
| 15 | `bad` / `goodB2G` | in-range-but-truncating quotient, e.g. `data = 3.0f` -> `33.333...` | truncation toward zero: prints `33`; negative `data = -3.0f` -> `-33` (toward zero, not floor) |
| 16 | `printIntLine` | out-of-range enum/int abuse across FFI: `INT_MIN`, `INT_MAX`, `-1`, `0`, and a value with no meaning to the API | `printf("%d\n", n)` prints the value verbatim; no validation, no rejection |
| 17 | `driver` | *both* arguments simultaneously invalid (`goodData` rejected by the guard **and** `badData == 0`) — checks the guards are independent and output ordering is preserved | 6 lines: `Calling good()...`, `50`, the divide-by-zero message, `Finished good()`, `Calling bad()...`, `-2147483648`, `Finished bad()` |

Row status: see `tests/error_paths.rs`; every row above has a dedicated
differential test asserting the C and Rust `.so` produce **byte-identical**
stdout (the only observable, since all functions are `void`).

| row | test | status |
|-----|------|--------|
| 1  | `err_01_print_line_null`              | [x] |
| 2  | `err_02_print_line_empty`             | [x] |
| 3  | `err_03_good_zero`                    | [x] |
| 4  | `err_04_good_negative_zero`           | [x] |
| 5  | `err_05_good_tiny_but_finite`         | [x] |
| 6  | `err_06_good_threshold_exact`         | [x] |
| 7  | `err_07_good_nan`                     | [x] |
| 8  | `err_08_good_one_step_past_threshold` | [x] |
| 9  | `err_09_bad_zero`                     | [x] |
| 10 | `err_10_bad_negative_zero`            | [x] |
| 11 | `err_11_bad_nan`                      | [x] |
| 12 | `err_12_bad_overflow_positive`        | [x] |
| 13 | `err_13_bad_overflow_negative`        | [x] |
| 14 | `err_14_bad_infinity`                 | [x] |
| 15 | `err_15_truncation_toward_zero`       | [x] |
| 16 | `err_16_print_int_line_extremes`      | [x] |
| 17 | `err_17_driver_both_invalid`          | [x] |

## Verification evidence

All 17 rows pass as differential tests in `tests/error_paths.rs` (21 tests total —
the 17 rows plus 4 generic FFI-boundary tests), under both the `debug` and
`release` profiles and all three feature sets. Confirmed via
`cargo test --test error_paths -- --test-threads=1`.

Each row uses `diff_exact`, which does two things: it asserts the C and Rust byte
streams are equal, **and** it pins the exact sentinel text the C produces (e.g.
`"-2147483648\n"`, `"This would result in a divide by zero\n"`, `""`). So a row
cannot pass merely because "both sides failed somehow" — the specific rejection is
checked.

Additional generic boundary coverage beyond the table
(`generic_*` tests in `tests/error_paths.rs`):

* `generic_print_line_pointer_shapes` — NULL, pointer at a NUL byte, pointer into
  the middle of a buffer past a NUL, and lengths straddling the 4 KiB/8 KiB stdio
  buffer boundaries.
* `generic_out_of_range_int_values` — 129 probes covering every power of two and
  its neighbours at both signs, plus `INT_MIN`/`INT_MAX` and their neighbours.
  This is the out-of-range-enum class: `printIntLine` takes a plain `int`, so a C
  enum value with no valid variant is just another `int` and must round-trip
  identically. Verified it does.
* `generic_float_exponent_sweep` — all 256 exponent values x 4 mantissas x both
  signs = 2048 floats through **both** `bad` and `good`, covering every zero,
  subnormal, normal, infinity and NaN encoding class.
* `generic_statelessness_across_repeated_calls` — zero calls, and 100 / 50
  repeated calls, proving neither library carries hidden state between calls.

The cast boundary in row 12/13 was confirmed to be genuinely straddled, not
merely nearby: with `x = nextafter(100/2^31)` the C prints `2147483484` (in
range), while at `x = 100/2^31` exactly the quotient is `2^31` and the C prints
`-2147483648`. Rust's plain `as` cast would have *saturated* to `2147483647`
there, so `to_c_int`'s explicit range check is load-bearing and is exercised.
