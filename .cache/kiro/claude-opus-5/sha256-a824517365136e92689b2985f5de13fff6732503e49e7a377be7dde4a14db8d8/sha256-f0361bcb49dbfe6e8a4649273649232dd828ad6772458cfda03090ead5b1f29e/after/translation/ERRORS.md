# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Every rejection site was located
with:

```
grep -n -E 'return\s+(-1|0|NULL)|assert|RETURN_ERROR|errno|exit\(' c_src/src/lib.c
grep -n -E '\bif\b|\bswitch\b|#if|continue|break'                 c_src/src/lib.c
```

Results: **6 early-return sites** (lines 41, 63, 83, 97, 106, 115), each with a
two-term `||` guard → **12 distinct rejection conditions**, plus **2 skip
(`continue`) conditions** at line 69. There are **no** `assert`s, no
`RETURN_ERROR`-style macros, no error enums, no `errno` use, and no `exit()`.

## Reachability note (important, and the reason this table is shaped this way)

The public ABI is exactly one function:

```c
int memchra2(int a, int b, int c, int d);
```

It takes four plain `int`s — **no pointers, no lengths, no enums, no
out-parameters**. Every guarded helper is `static` and is called by `memchra2`
with compile-time-fixed, always-valid arguments (a 64-byte local array that
`snprintf` always fills with at least `"test"`, a 4-element `int` array, string
literals, `count = 4`, `len = 4 == sizeof(int)`). Consequently **`memchra2`
itself has no error return: it produces a defined result for every one of the
2^128 input tuples.**

Rows below are therefore split into two groups:

- **E1–E14** — the C code's intrinsic rejection conditions. Each row records the
  trigger, the C result, and how the row is *observable*. Rows whose guard is
  unreachable from the public ABI are verified by asserting that the Rust
  `.so` takes the same *non*-rejecting path as the C `.so` (i.e. the guard must
  be present-but-not-taken in both; if the Rust translation got a guard
  backwards or dropped a `continue`, `memchra2`'s return value changes and the
  differential test fails). The observable consequence is listed per row.
- **B1–B7** — the generic FFI-boundary inputs that *are* reachable: extremes of
  the `int` domain, one-step-past values around each internal value branch, and
  reinterpretation across the FFI boundary. Since no parameter is a pointer,
  length, or enum, these are the complete analogue of the
  "null / zero length / oversized length / out-of-range enum" checklist.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | observable consequence used by the test |
|---|----------|---------------------------------------------|-------------------|------------------------------------------|
| E1 | `process_buffer` (line 40) | `buffer == NULL` | `return -1` | unreachable: caller passes `char buffer[64]`. Guard must not fire → `buf_sum > 0` holds and `result += buf_sum % 256`. If Rust fired it, `result` would lose the `% 256` term. |
| E2 | `process_buffer` (line 40) | `*buffer == '\0'` (non-NULL but empty string) | `return -1` | unreachable: `snprintf` always writes `"test…"`, so `buffer[0] == 't'`. Same observable as E1. |
| E3 | `process_strings` (line 62) | `strings == NULL` | `return 0` | unreachable: caller passes a 4-element array of literals. Guard must not fire → `matches == 3`, contributing `+15`. |
| E4 | `process_strings` (line 62) | `count <= 0` | `return 0` | unreachable: caller passes `count = 4`. Same observable as E3. |
| E5 | `process_strings` (line 69) | `*i == NULL` (a NULL element mid-array) | `continue` (element skipped, not counted) | unreachable: all 4 elements are non-NULL literals. Skip must not fire → `matches == 3` (`+15`). |
| E6 | `process_strings` (line 69) | `**i == '\0'` (an empty-string element) | `continue` (element skipped) | unreachable: no element is empty. Same observable as E5. |
| E7 | `safe_sum_array` (line 82) | `arr == NULL` | `return 0` | unreachable: caller passes `int values[4]`. Guard must not fire → `result += a+b+c+d` (wrapping). Detectable at every input. |
| E8 | `safe_sum_array` (line 82) | `size == 0` | `return 0` | unreachable: caller passes `size = 4`. Same observable as E7. |
| E9 | `interpret_as_int` (line 96) | `bytes == NULL` | `return 0` | unreachable: caller passes `unsigned char bytes[4]`. Guard must not fire → `result ^= LE32(b&0xFF, c&0xFF, d&0xFF, 0)`. |
| E10 | `interpret_as_int` (line 96) | `len < sizeof(int)` i.e. `len ∈ {0,1,2,3}` | `return 0` | unreachable: caller passes `len = 4`, exactly the boundary that *passes*. `len == 4` is the one-step-inside value; `len == 3` is the one-step-outside value. Same observable as E9. |
| E11 | `count_occurrences` (line 105) | `text == NULL` | `return 0` | unreachable: caller passes `buffer`. Guard must not fire → `result += dash_count * 10`. |
| E12 | `count_occurrences` (line 105) | `*text == '\0'` | `return 0` | unreachable: `buffer[0] == 't'`. Same observable as E11. |
| E13 | `complex_iteration` (line 114) | `data == NULL` | `return -1` | unreachable: caller passes `values`. Guard must not fire → `result += xor of low bytes`; if Rust fired it, `result` would be off by (xor + 1). |
| E14 | `complex_iteration` (line 114) | `count == 0` | `return -1` | unreachable: caller passes `count = 4`. Same observable as E13. |
| B1 | `memchra2` | `a = b = c = d = INT_MIN` (`-2147483648`) — most negative, and the value whose negation overflows | defined value, no rejection; `snprintf` writes 4 × 11-char signed decimals (`4 + 44 + 3 = 51 < 64`, so still no truncation) | direct return-value comparison |
| B2 | `memchra2` | `a = b = c = d = INT_MAX` (`2147483647`) — most positive | defined value, no rejection | direct return-value comparison |
| B3 | `memchra2` | `a = b = c = d = 0` — the "zero length"/empty analogue; `int_to_float_bits(0) == +0.0f` fails `f > 0.0f` | defined value; float term omitted | direct return-value comparison |
| B4 | `memchra2` | `a` one step past each side of the float acceptance window: `a ∈ {0, 1, 1148846079, 1148846080}` (`1148846080 == 0x447A0000 == bits(1000.0f)`) | `f > 0.0f && f < 1000.0f` true only for `1 … 1148846079`; `(int)f` truncation applies | direct return-value comparison |
| B5 | `memchra2` | `a` = bit patterns that are **not finite positive floats**: `0x7F800000` (`+inf`), `0xFF800000` (`-inf`), `0x7FC00000` (quiet NaN), `0x7F800001` (signalling NaN), `0x80000000` (`-0.0f`) | NaN makes both comparisons false; `±inf`/`-0.0` also rejected → float term omitted. No trap, no error. | direct return-value comparison |
| B6 | `memchra2` | `a` = smallest positive subnormal `1` (`≈1.4e-45`) — passes `f > 0.0f` yet `(int)f == 0` | float term contributes `0`, not a rejection | direct return-value comparison |
| B7 | `memchra2` | symbol invoked through a **differently-typed FFI signature**: `extern "C" fn(u32,u32,u32,u32) -> u32` instead of `(i32,…) -> i32` (the "out-of-range value with no valid variant" analogue — `int` has no invalid bit pattern, so the boundary case is unsigned reinterpretation of the same 4 words) | identical 32-bit result word from both `.so`s | raw 32-bit words compared |

## Checklist

- [x] E1 — `phase_c_errors::e1_e2_process_buffer_guards`
- [x] E2 — `phase_c_errors::e1_e2_process_buffer_guards`
- [x] E3 — `phase_c_errors::e3_e6_process_strings_guards`
- [x] E4 — `phase_c_errors::e3_e6_process_strings_guards`
- [x] E5 — `phase_c_errors::e3_e6_process_strings_guards`
- [x] E6 — `phase_c_errors::e3_e6_process_strings_guards`
- [x] E7 — `phase_c_errors::e7_e8_safe_sum_array_guards`
- [x] E8 — `phase_c_errors::e7_e8_safe_sum_array_guards`
- [x] E9 — `phase_c_errors::e9_e10_interpret_as_int_guards`
- [x] E10 — `phase_c_errors::e9_e10_interpret_as_int_guards`
- [x] E11 — `phase_c_errors::e11_e12_count_occurrences_guards`
- [x] E12 — `phase_c_errors::e11_e12_count_occurrences_guards`
- [x] E13 — `phase_c_errors::e13_e14_complex_iteration_guards`
- [x] E14 — `phase_c_errors::e13_e14_complex_iteration_guards`
- [x] B1 — `phase_c_errors::b1_int_min_extremes`
- [x] B2 — `phase_c_errors::b2_int_max_extremes`
- [x] B3 — `phase_c_errors::b3_all_zero`
- [x] B4 — `phase_c_errors::b4_float_window_boundaries`
- [x] B5 — `phase_c_errors::b5_non_finite_float_bit_patterns`
- [x] B6 — `phase_c_errors::b6_smallest_subnormal`
- [x] B7 — `phase_c_errors::b7_unsigned_ffi_signature`
