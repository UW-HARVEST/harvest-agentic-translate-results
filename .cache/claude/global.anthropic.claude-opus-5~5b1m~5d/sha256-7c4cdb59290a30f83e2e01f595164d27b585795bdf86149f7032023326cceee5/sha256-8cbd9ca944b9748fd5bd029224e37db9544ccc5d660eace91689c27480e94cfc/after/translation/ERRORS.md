# ERRORS.md — Phase A: error-surface table

## Mechanical derivation

Every rejection/error construct was grepped out of the complete C source
(`c_src/src/driver.c`, `c_src/include/driver.h`):

```
grep -nE 'return|assert|NULL|errno|exit|abort|if|switch|#if|#ifdef|==|!=|ERROR|-1' \
    src/driver.c include/driver.h
```

The only matches outside comments are `#include <stdint.h>`, `#include <stdio.h>`
and the header's `#ifndef DRIVER_H_` / `#endif` include guard.

Counts across the whole library:

| construct | occurrences |
|-----------|-------------|
| `return` statements | 0 |
| error-return macros (`RETURN_ERROR`-style) | 0 |
| `return -1` / `return NULL` / error enums | 0 |
| `assert` / `static_assert` | 0 |
| explicit range / bounds checks | 0 |
| null-pointer checks | 0 |
| MIN/MAX constants | 0 |
| `errno` writes, `exit`, `abort` | 0 |
| `if` / `switch` / `?:` branches | 0 |
| `#ifdef` feature branches (outside the include guard) | 0 |

`driver` is `void`-returning, takes one by-value `double`, has **no pointer
parameters, no enum parameters, no length/count parameters, and no failure
mode**. Every `double` bit pattern is a valid input; the function is total.

## The error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | `driver` | *(none — the C source contains zero rejection paths)* | n/a |

There are **zero rows**: the C library rejects nothing. Recording a row here
would mean inventing an error the C does not have.

## Substitute error-path coverage (the generic-boundary gate)

Because the table is empty, the Phase C obligation is redischarged against the
*generic* boundaries the task requires for every C API, reinterpreted for this
signature. Each is an input the C accepts and handles, so the assertion is
"C and Rust produce **byte-identical stdout**", which is this API's analogue of
"the same error code or sentinel" — a divergence here is exactly the
happy-path-invisible class of bug the phase targets.

| # | boundary class | concrete input(s) | test | status |
|---|----------------|-------------------|------|--------|
| E1 | null pointer | *not applicable* — `driver` has no pointer parameter. The format string is a private `const` in both implementations and is never caller-supplied. | `err_no_pointer_parameters` (documents + asserts the by-value ABI via a `*const c_void`-free call) | ✅ |
| E2 | zero length | *not applicable* — no length/size/count parameter. Nearest analogue: the additive and multiplicative identities and zeroes. `+0.0`, `-0.0`, `1.0` | `err_zero_and_signed_zero` | ✅ |
| E3 | oversized length | *not applicable* — no length parameter. Nearest analogue: magnitudes that make `%.4f` emit a maximal (~310-char) field. `DBL_MAX`, `-DBL_MAX`, `0x1.fffffffffffffp+1023`, `1e308` | `err_oversized_field_width` | ✅ |
| E4 | one step past the valid range (upper) | `nextafter(DBL_MAX, +inf)` == `+inf`; `f64::INFINITY`; `f64::NEG_INFINITY` | `err_one_past_range_infinities` | ✅ |
| E5 | one step past the valid range (lower / underflow) | `nextafter(0.0, 1.0)` == min subnormal `5e-324`; `-5e-324`; `nextafter(DBL_MIN, 0.0)` == max subnormal; `DBL_MIN` itself | `err_one_past_range_subnormals` | ✅ |
| E6 | out-of-range enum value across the FFI boundary | *not applicable* — `driver` has no enum parameter. The exhaustive analogue for a `double` parameter is the *non-numeric* bit patterns that have no valid "variant": every NaN class. `+qNaN`, `-qNaN`, `+sNaN` (`0x7ff0000000000001`), `-sNaN`, NaN with a payload (`0x7ff8deadbeefcafe`), `0x7fffffffffffffff`, `0xffffffffffffffff` | `err_nan_variants_no_valid_variant` | ✅ |
| E7 | exhaustive out-of-range sweep | all 2^11 exponent fields × representative mantissas, i.e. every IEEE-754 *class* (zero, subnormal, normal, inf, NaN) in both signs, built by reinterpreting raw `u64`s — including patterns no numeric literal can name | `err_all_ieee754_classes` | ✅ |
| E8 | rounding-mode / tie boundary (silent-wrong-answer class) | `%.4f` half-way cases where round-half-to-even vs round-half-away diverge: `0.00005`, `0.00015`, `2.00005`, `1.0000500000000001`, `0.5/10000` neighbourhoods, and `nextafter` neighbours of each | `err_rounding_ties` | ✅ |
| E9 | re-entrancy / interleaving with the caller's own C stdout | call `driver` many times in sequence and interleave with direct `printf` from the test, asserting the combined stream matches | `err_interleaved_with_caller_stdout` | ✅ |

All nine boundary tests are differential (both `.so`s loaded with `libloading`,
stdout captured per call and compared byte-for-byte).
