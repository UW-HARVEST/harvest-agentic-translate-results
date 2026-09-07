# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical grep evidence

```
$ grep -n 'return\|assert\|NULL\|errno\|ERROR\|-1\|exit(' src/lib.c include/lib.h
(no matches)
```

The C function is `void`-returning and performs **zero** input validation:
no `return` statements, no error macros, no error enum, no `assert`, no null
check, no range check, no `errno` use, no min/max constant. Therefore there is
**no error code / sentinel surface** to compare — the only observable result is
the bytes written to `dest` (and whether anything is written at all).

Consequently the rows below enumerate every *rejection-like / degenerate*
condition the C actually branches on, i.e. every path on which the function
declines to produce a normalized vector. Each row's "expected C result" is the
exact observable behaviour, which the Rust must reproduce byte-for-byte.

Full branch inventory from the source (line numbers are `src/lib.c`):

```
 9:    for (i = 0; i < size; i++)          -> loop guard: size <= 0 skips
11:    if (sum > 0.0f) {                   -> false for sum == 0.0 and for NaN
15:    } else if (dest != src) {           -> pointer identity check
16:        memset(dest, 0, size * sizeof(float));   -> int -> size_t conversion
```

## Error / rejection surface

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `normalize` | `size == 0`, `dest != src` — loop guard `i < size` false at line 9, `sum` stays `0.0f`, `sum > 0.0f` false, `dest != src` true, `memset(dest, 0, 0)` | writes nothing (0-byte memset); no bytes of `dest` change; no crash | `err_row01_size_zero_distinct` | [x] |
| 2 | `normalize` | `size == 0`, `dest == src` (in place) | writes nothing; buffer unchanged | `err_row02_size_zero_aliased` | [x] |
| 3 | `normalize` | `size < 0`, `dest == src` (in place) — loop skipped, `sum == 0.0f`, `else if (dest != src)` **false**, so the dangerous `memset` is NOT reached | writes nothing; buffer unchanged; no crash for any negative `size` incl. `INT_MIN` | `err_row03_negative_size_aliased` | [x] |
| 4 | `normalize` | `size < 0`, `dest != src` — `memset(dest, 0, (size_t)size * 4)` where `(size_t)(-1) * 4 == 0xFFFFFFFFFFFFFFFC` | **guaranteed SIGSEGV / heap destruction in C** (astronomically large memset). Not exercised in-process; the *length computation* is verified instead to be bit-identical: Rust `(size as i64 as u64).wrapping_mul(4)` == C `(size_t)size * sizeof(float)` for all `int`. See `err_row04_negative_size_length_arithmetic` (arithmetic-equivalence proof, no call) | `err_row04_negative_size_length_arithmetic` | [x] |
| 5 | `normalize` | all-zero input, `dest != src` — `sum == 0.0f`, `sum > 0.0f` false | `dest[0..size]` zero-filled via `memset` | `err_row05_all_zero_distinct` | [x] |
| 6 | `normalize` | all-zero input, `dest == src` — `sum == 0.0f` and `dest != src` false | **nothing written at all** (buffer left as-is) | `err_row06_all_zero_aliased` | [x] |
| 7 | `normalize` | input containing `NaN` → `sum` becomes `NaN`; `NaN > 0.0f` is false | falls into the `else if`: zero-fill when `dest != src`, no-op when `dest == src` (i.e. NaN is *rejected*, not propagated through the divide) | `err_row07_nan_input` | [x] |
| 8 | `normalize` | input whose squares all underflow to `0.0f` (e.g. `1e-30f`, `FLT_TRUE_MIN`) → `sum == 0.0f` despite non-zero input | zero-fill when `dest != src` / no-op when `dest == src` — the non-zero input is silently discarded | `err_row08_underflow_to_zero_sum` | [x] |
| 9 | `normalize` | input containing `-0.0f` only → `sum = (-0.0)*(-0.0) = +0.0`, `0.0 > 0.0` false | same `else if` path (zero-fill / no-op) | `err_row09_negative_zero` | [x] |
| 10 | `normalize` | input with a magnitude whose square overflows (`>= ~1.845e19f`) → `sum == +inf`, `+inf > 0.0f` **true**, `1.0f/sqrtf(inf) == 0.0f` | `dest[i] = src[i] * 0.0f` → `±0.0` for finite `src[i]`, and default QNaN (`0xFFC00000`) for `±inf` elements | `err_row10_sum_overflow_inf` | [x] |
| 11 | `normalize` | input containing `±inf` → `sum == +inf` | as row 10: `inf * 0.0f` yields x86 default QNaN, finite neighbours yield signed zero | `err_row11_inf_input` | [x] |
| 12 | `normalize` | `dest == src` exactly (full aliasing) with `sum > 0.0f` | in-place normalize; reads of `src[i]` happen after the write of `dest[i-1]` but the indices coincide so the result equals the out-of-place result | `err_row12_aliased_normalizing` | [x] |
| 13 | `normalize` | partially overlapping `dest`/`src` (`dest = src + k`) with `sum > 0.0f` — C has no `restrict`, so the sequential read-after-write cascade is the defined observable behaviour | element-wise sequential cascade: later reads see earlier writes | `err_row13_partial_overlap` | [x] |
| 14 | `normalize` | `size == 1` with a value whose square is exactly representable (e.g. `1.0f`, `2.0f`) vs one where `1.0f/sqrtf(v*v) != 1.0f/|v|` | the double-rounding of `sqrtf` then reciprocal must match exactly (result is often *not* exactly `±1.0f`) | `err_row14_size_one_rounding` | [x] |
| 15 | `normalize` | out-of-range "enum" / bogus scalar across FFI: `size` values `INT_MAX`, `INT_MIN`, `-1` passed with `dest == src` (the only in-process-safe way to feed extreme `size`) | `INT_MAX` with `dest == src` would read out of bounds → not called; `INT_MIN`/`-1` with `dest == src` → no-op. `size` is a plain `int` with no valid-range check, so every bit pattern is "accepted"; only the loop guard distinguishes them | `err_row15_extreme_size_aliased` | [x] |
| 16 | `normalize` | null `dest` and null `src` with `size == 0` — no dereference occurs (loop skipped, `memset(NULL, 0, 0)`) | no crash, no write; `dest != src` is false when both are `NULL`, so not even a 0-byte `memset` is issued | `err_row16_null_pointers_size_zero` | [x] |
| 17 | `normalize` | null `dest`, non-null `src`, `size == 0` — `dest != src` true → `memset(NULL, 0, 0)` | no crash (0-length `memset` on `NULL` does not touch memory) | `err_row17_null_dest_size_zero` | [x] |

Rows 4 and the `INT_MAX` part of row 15 are the only conditions that cannot be
executed in-process because the **C itself** would segfault or read unmapped
memory; for row 4 the divergence-prone part (the `size_t` length arithmetic) is
verified exhaustively instead.
