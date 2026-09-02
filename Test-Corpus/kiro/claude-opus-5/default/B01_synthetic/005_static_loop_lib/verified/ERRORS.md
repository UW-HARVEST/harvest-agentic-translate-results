# ERRORS.md — Error-surface table (Phase A / Phase C)

## Mechanical derivation

Every rejection/error construct was searched for in the complete C source
(`c_src/src/staticloop.c`, `c_src/include/staticloop.h` — the only C files):

```sh
grep -nE "assert|NULL|ERROR|RETURN_ERROR|errno|return *-1|goto|exit|abort" c_src/src/*.c c_src/include/*.h
grep -n  "return" c_src/src/*.c c_src/include/*.h
grep -nE "if *\(|switch|#ifdef|#if |MAX|MIN" c_src/src/*.c c_src/include/*.h
```

Results (verbatim, comment/licence lines excluded):

- `return` statements: exactly two — `staticloop.c:31 return sum;` and
  `staticloop.c:42 return;` (a bare `void` return).
- `assert` / `NULL` / `ERROR` / `RETURN_ERROR` / `errno` / `return -1` / `goto` /
  `exit` / `abort`: **0 matches**.
- conditionals: exactly one — `staticloop.c:39 for (int i = 0; i < 10; i++)`,
  a fixed trip-count loop bound, not an input validation check.
- `#ifdef` / `#if` (other than the header's include guard): 0 matches.
- min/max constants, range checks, null checks: 0 matches.
- pointer parameters anywhere in the public API: 0 (`int static_sum(int)`,
  `void driver(int)`).
- enum types anywhere in the public API: 0.

**Conclusion: the C library has an EMPTY explicit error surface.** It validates
nothing, has no error codes, no sentinel return values, and no reserved
parameter values. `static_sum` returns the accumulator, and every one of the
2^32 `int` values is a legitimate return value — so no return value can be
interpreted as "error". `driver` returns `void`. Both accept the full `int`
domain. There are therefore **zero rejection rows** to derive.

## Error-surface rows

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | *(no explicit rejection exists anywhere in the C source)* | — |

## Generic-boundary rows (mandated coverage, tested even though the C rejects nothing)

Because the table above is empty, Phase C is discharged by proving that the C
accepts these boundary inputs *without* erroring and that the Rust behaves
identically (same returned value / same stdout bytes / no panic, no abort, no
trap). A Rust `debug_assert`, overflow panic, or `unimplemented!()` on any of
these rows would be a divergence, so these are genuine error-path tests.

| # | function | boundary condition constructed | expected C result | test | ✓ |
|---|----------|--------------------------------|-------------------|------|:-:|
| B1 | `static_sum` | `update = 0` (zero-length/no-op input) | returns accumulator unchanged; no error | `error_paths::b1_zero_update` | [x] |
| B2 | `static_sum` | `update = INT_MAX` (max in-range value) | returns `sum + INT_MAX`, two's-complement wrap; no error | `error_paths::b2_int_max_update` | [x] |
| B3 | `static_sum` | `update = INT_MIN` (min in-range value) | returns `sum + INT_MIN`, two's-complement wrap; no error | `error_paths::b3_int_min_update` | [x] |
| B4 | `static_sum` | `update = -1` (the classic C error sentinel, here a *valid* input) | returns `sum - 1`; **must not** be treated as an error | `error_paths::b4_minus_one_sentinel` | [x] |
| B5 | `static_sum` | signed-overflow past the valid `int` range: accumulator driven to `INT_MAX` then `update = 1` (one step past the documented range) | wraps to `INT_MIN`; no trap/abort | `error_paths::b5_overflow_one_past_max` | [x] |
| B6 | `static_sum` | signed-underflow one step past `INT_MIN`: accumulator at `INT_MIN` then `update = -1` | wraps to `INT_MAX`; no trap/abort | `error_paths::b6_underflow_one_past_min` | [x] |
| B7 | `driver` | `stride = 0` (degenerate/empty-effect stride) | prints the current accumulator 10× ; no error | `error_paths::b7_driver_zero_stride` | [x] |
| B8 | `driver` | `stride = INT_MAX` — makes the internal `i * stride` overflow on every `i >= 2` | wraps; prints 10 lines; no trap/abort | `error_paths::b8_driver_int_max_stride` | [x] |
| B9 | `driver` | `stride = INT_MIN` — `i * stride` overflow, negative direction | wraps; prints 10 lines; no trap/abort | `error_paths::b9_driver_int_min_stride` | [x] |
| B10 | `driver` | `stride = -1` (negative "oversized-length"-analogue / sentinel) | prints 10 descending sums; no error | `error_paths::b10_driver_minus_one` | [x] |
| B11 | `driver` | `stride` chosen so the *accumulated* `sum` overflows mid-loop (`stride = INT_MAX/8`) | wraps mid-loop; still prints exactly 10 lines | `error_paths::b11_driver_sum_overflow_midloop` | [x] |
| B12 | both | out-of-range *enum* value across the FFI boundary | **N/A — the public API declares no enum type** (verified by grep: 0 `enum` in `c_src`). The `int` domain is fully covered by B1–B11 + Phase B randomization, so every representable bit pattern of every parameter is a tested input. | — | [x] |
| B13 | both | null pointer arguments | **N/A — the public API takes no pointer arguments** (verified by grep: `int static_sum(int)`, `void driver(int)`). | — | [x] |
| B14 | both | zero / oversized *lengths* | **N/A — no length, size, count, or buffer parameter exists.** The nearest analogues are B1 (zero) and B2/B3/B8/B9 (extremal magnitudes). | — | [x] |

Rows B1–B11 are exercised as differential tests in
`translation/tests/error_paths.rs`; each asserts C and Rust agree on the exact
returned `int` (not merely "both succeeded") and, for `driver`, on the exact
stdout byte stream.
