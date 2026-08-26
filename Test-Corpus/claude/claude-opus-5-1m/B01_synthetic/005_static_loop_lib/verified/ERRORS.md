# ERRORS.md — Error-surface table (Phase A / Phase C)

Mechanically derived from the **complete** C source (`c_src/src/staticloop.c`,
43 lines, and `c_src/include/staticloop.h`). Greps run over all of `c_src/`:

```
$ grep -nE 'return|assert|NULL|errno|RETURN_ERROR|exit\(|abort\(' c_src/src/staticloop.c
31:  return sum;
42:  return;
$ grep -rnE 'assert|NULL|errno|-1|EINVAL|#define .*ERR' c_src/src/staticloop.c c_src/include/staticloop.h
(no matches)
$ grep -rnE '\bif\b|\bswitch\b|#if|#ifdef' c_src/src/staticloop.c
(no matches — only the `for` loop in driver())
```

## Result: the C library has an EMPTY error surface

* Neither public function returns an error code or sentinel.
  * `static_sum` returns the running total (`return sum;`) — *every* `int` is a
    legal return value, and no value is reserved to mean "error".
  * `driver` returns `void` (`return;`).
* There is **no** `assert`, no `NULL` check, no range check, no `errno` use, no
  error enum, no `RETURN_ERROR`-style macro, no `exit`/`abort`, and **no `if`
  or `switch` statement at all**.
* Neither function takes a pointer, so there is no null-pointer input to reject.
* Neither function takes an enum, so there is no out-of-range enum variant to
  reject (documented below anyway, because every `int` is accepted verbatim).
* Neither function takes a length/size/count, so there is no zero-length or
  oversized-length rejection. `driver`'s trip count is the hard-coded literal
  `10` (`for (int i = 0; i < 10; i++)`), not caller-controlled.

Consequently the table has **zero rows of the form "C rejects input X"**. To
keep the phase meaningful, the rows below enumerate every *boundary /
degenerate* input the C code accepts **without** rejecting, i.e. the exact
inputs that would make a naively-written Rust translation *panic* (arithmetic
overflow in debug, `unwrap`, etc.) instead of returning what C returns. Each row
is covered by a differential test that asserts C and Rust produce the **same**
value (and the same `driver` stdout bytes) rather than "both failed somehow".

The relevant C statements are:

```c
static int sum = 0;
sum += update;          /* line 30 — signed overflow is UB in C; the -O0 build
                                     wraps two's-complement (verified in the
                                     disassembly: plain `add %edx,%eax`)      */
...
static_sum(i * stride)  /* line 40 — same for the multiply: plain `imul`      */
```

## Error / rejection / boundary table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| E1 | `static_sum` | no rejection path exists at all: any `int update`, including values with no "valid range" | never errors; always returns the new running total; no errno/sentinel | `err_e1_no_rejection_path_any_int` | [x] |
| E2 | `static_sum` | `update = INT_MAX` applied to a positive running total ⇒ signed-overflow (UB in C) | `-O0` C wraps two's-complement; returns wrapped `int`, no trap/abort | `err_e2_static_sum_overflow_int_max` | [x] |
| E3 | `static_sum` | `update = INT_MIN` applied to a negative running total ⇒ signed-underflow (UB in C) | `-O0` C wraps two's-complement; returns wrapped `int`, no trap/abort | `err_e3_static_sum_underflow_int_min` | [x] |
| E4 | `static_sum` | repeated `update = INT_MAX` — accumulator wraps many times in a row | wraps each time; no saturation, no error | `err_e4_static_sum_repeated_overflow` | [x] |
| E5 | `static_sum` | `update = 0` (degenerate "empty" update) | returns the unchanged running total | `err_e5_static_sum_zero_update` | [x] |
| E6 | `driver` | `stride = INT_MAX` ⇒ `i * stride` overflows for every `i >= 2` (UB in C) | `-O0` C wraps the `imul`; prints 10 wrapped lines, returns normally | `err_e6_driver_stride_int_max` | [x] |
| E7 | `driver` | `stride = INT_MIN` ⇒ `i * stride` overflows for every `i >= 2` (UB in C) | `-O0` C wraps the `imul`; prints 10 wrapped lines, returns normally | `err_e7_driver_stride_int_min` | [x] |
| E8 | `driver` | `stride` one step past the largest overflow-free value (`INT_MAX/9 + 1 = 238609295`), and the largest safe value itself (`238609294`) | no rejection; the +1 case wraps only at `i = 9`, the safe case never wraps | `err_e8_driver_stride_one_past_safe_range` | [x] |
| E9 | `driver` | `stride = 0` (degenerate: every update is 0) | prints the current total 10 times; no error | `err_e9_driver_stride_zero` | [x] |
| E10 | `driver` | `stride` negative (e.g. `-1`), i.e. "out of range" for a stride | accepted verbatim, no sign check; total walks downward | `err_e10_driver_negative_stride` | [x] |
| E11 | both | value passed across FFI that has no valid "variant" — the C prototypes take plain `int`, so an out-of-range enum-like value (`0x7FFF_FFFF`, `-0x8000_0000`, `0xDEAD_BEEF as i32`) is simply an `int` | accepted verbatim; no validation, no default branch, no error | `err_e11_out_of_range_enum_like_values` | [x] |
| E12 | both | `driver` cannot be asked for 0 or an oversized iteration count (trip count is the literal `10`); passing extreme `stride` never changes the line count | exactly 10 lines of output for every `stride` | `err_e12_driver_always_ten_lines` | [x] |
| E13 | both | no pointer parameters ⇒ null-pointer input is impossible; verified from the header signatures | n/a — asserted structurally (signatures take `int` only) | `err_e13_no_pointer_parameters` | [x] |
| E14 | `static_sum` | boundary crossing 0: state at `INT_MIN`, `update = -1`; and state at `INT_MAX`, `update = 1` | wraps to `INT_MAX` / `INT_MIN` respectively | `err_e14_static_sum_wrap_at_both_ends` | [x] |

All 14 rows have a passing differential test in
`tests/differential.rs` (module `errors`), each of which calls **both** the C
`.so` and the Rust `.so` through `libloading` and asserts the returned `int`
(and, for `driver`, the captured stdout bytes) are identical.

## Generic boundaries covered even though the table has no rejection rows

| generic boundary | how it is covered here |
|------------------|------------------------|
| null pointers | impossible: no parameter is a pointer (asserted structurally against the header in `err_e13_no_pointer_parameters`) |
| zero length | no length parameter exists; the closest analogues — `update = 0` and `stride = 0` — are rows E5 / E9 |
| oversized length | no length parameter exists; the loop trip count is the literal `10` — row E12 asserts it can never change |
| one step past a documented valid range | `INT_MAX`/`INT_MIN` and `INT_MAX ± 1`-class neighbours (rows E2, E3, E14) and the `driver` multiply boundary `INT_MAX/9` and `INT_MAX/9 + 1` (row E8) |
| out-of-range enum values across FFI | row E11: both prototypes take plain `int`, so bit patterns such as `0xDEAD_BEEF`, `0xCAFE_BABE`, `0x8000_0000` are legal inputs and are accepted verbatim by both libraries |
| unsigned/signed confusion | row E11 passes values whose `u32` interpretation differs from their `i32` interpretation |

## Result

`static_sum` and `driver` never reject anything, so "both return the same error"
degenerates to "both return the same value / same bytes", which is exactly what
each row asserts. No divergence was found: **14/14 rows pass** in every
configuration (see `CONFIGS.md`).
