# ERRORS.md — Error-surface table (Phase A / Phase C)

## How this table was derived

Mechanical scan of the **entire** C source (`c_src/src/driver.c`,
`c_src/include/driver.h`) for every rejection / error construct. Comments are
stripped first, then the remainder is tokenised on non-identifier characters so
that `#include` cannot be mistaken for `if`:

```sh
# strip // comments, then look for any rejection/error/branch keyword
sed 's://.*::' c_src/src/driver.c \
  | grep -nowE "return|RETURN_ERROR|assert|NULL|nullptr|errno|exit|abort|goto|if|else|switch|case|while|for|raise|longjmp|perror|strerror"
# -> no output

# and for conditional/comparison operators
sed 's://.*::' c_src/src/driver.c | grep -nE '\?|==|!=|<=|>=|&&|\|\|'
# -> no output
```

This exact scan is executed as a test, `error_e1_no_rejection_sites_exist`, so
the table cannot silently go stale: the test embeds the C file with
`include_str!` and fails if any of those constructs ever appears.

Two deliberate exclusions, for honesty about what the scan does and does not
prove:

* bare `<` and `>` are **not** searched for, because they occur only in
  `#include <stdint.h>` / `#include <stdio.h>` — never as comparisons;
* `#ifndef DRIVER_H_` / `#define` / `#endif` in the header are an include
  guard, i.e. a preprocessor construct, not a runtime check, and they generate
  no variant code (there is no `#else`).

The complete non-comment body of the library is 4 statements:

```c
typedef union { uint64_t x; double f; } raw_double_t;

void driver(double f) {
    raw_double_t u = {.f = f};
    printf("%llx %a %.4f\n", u.x, f, f);
}
```

**Result of the grep: the error surface is empty.**

* no `return` statement of any kind (the function is `void`)
* no error enum, no error code, no sentinel value, no out-parameter
* no `assert`, no `abort`, no `exit`
* no `NULL` check — the API takes **no pointer arguments**
* no range check, no `if`/`switch`/`?:` — the function is **branch-free** and
  **total**: it accepts all 2^64 bit patterns of a `double`
* no min/max constant, no length/size/count parameter
* no enum parameter, so there is no out-of-range-enum case
* `printf`'s return value is discarded, so even an I/O failure is not reported

Because `driver` cannot reject anything, Phase C cannot assert on "the same
error code". Instead, every row below asserts the equivalent property for a
total function: **for the degenerate / boundary / classically-invalid input,
C and Rust must both decline to fail and must emit byte-identical output.**
A divergence, a crash, a panic, or an abort in either implementation fails the row.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `driver` | *(none exists)* no `return`/`assert`/`NULL`/range check in the C source — the grep above yields zero rejection sites | function is total; never rejects, never returns a code | `error_e1_no_rejection_sites_exist` | [x] |
| E2 | `driver` | `f = +inf` (exponent all-ones, zero mantissa) — the classic "invalid" float | no error; prints `7ff0000000000000 inf inf` | `error_e2_infinity` | [x] |
| E3 | `driver` | `f = -inf` | no error; prints `fff0000000000000 -inf -inf` | `error_e3_infinity` | [x] |
| E4 | `driver` | `f = NaN` (quiet, default payload) | no error; prints `7ff8000000000000 nan nan` | `error_e4_quiet_nan` | [x] |
| E5 | `driver` | `f = -NaN` (quiet, sign bit set) — `%a`/`%.4f` must carry the sign | no error; prints `fff8000000000000 -nan -nan` | `error_e5_negative_quiet_nan` | [x] |
| E6 | `driver` | `f = signaling NaN` (exp all-ones, mantissa MSB **clear**, payload non-zero) — must not be quieted, `%llx` must show the original payload | no error; raw bits preserved verbatim | `error_e6_signaling_nan` | [x] |
| E7 | `driver` | `f = NaN` with arbitrary/maximal payloads (both signs, incl. mantissa `0x…FFFFF`, `0x…00001`) | no error; raw bits preserved verbatim | `error_e7_nan_payload_sweep` | [x] |
| E8 | `driver` | `f = -0.0` (negative zero: sign bit set, all other bits zero) — sign must survive into `%a` and `%.4f` | no error; prints `8000000000000000 -0x0p+0 -0.0000` | `error_e8_negative_zero` | [x] |
| E9 | `driver` | `f` = smallest positive subnormal (`bits == 1`, i.e. 5e-324) — "zero-like" underflow boundary | no error; `%a` emits a `0x0.…` leading digit, `%.4f` emits `0.0000` | `error_e9_min_subnormal` | [x] |
| E10 | `driver` | `f` = largest subnormal (`bits == 0x000FFFFFFFFFFFFF`) and smallest normal (`bits == 0x0010000000000000`) — one step either side of the subnormal/normal boundary | no error; `%a` leading digit flips `0` -> `1` | `error_e10_subnormal_normal_boundary` | [x] |
| E11 | `driver` | `f = ±f64::MAX` / one step below infinity (`bits == 0x7FEFFFFFFFFFFFFF`) — largest finite, `%.4f` expands to ~309 integer digits (oversized output, > glibc's internal buffer) | no error; full 309-digit expansion | `error_e11_max_finite_oversized_output` | [x] |
| E12 | `driver` | `f` one step **past** the largest finite value (`bits == 0x7FF0000000000000`) — i.e. the value immediately outside the finite range | no error; becomes `inf` | `error_e12_one_step_past_finite_range` | [x] |
| E13 | `driver` | `f` = value that lands exactly on the `%.4f` rounding cliff (`0.00005`, `-0.00005`) — round-half-even at the 4th decimal, output collapses to `0.0000`/`-0.0000` | no error; glibc's exact-tie rounding | `error_e13_rounding_cliff_ties` | [x] |
| E14 | `driver` | repeated invocation (state / stdout-buffering reuse): 4096 consecutive calls in one capture, including long lines that straddle glibc's `BUFSIZ` boundary | no error; no lost, truncated, or reordered output | `error_e14_repeated_calls_buffering` | [x] |
| E15 | `driver` | *(generic FFI boundary)* the C prototype has **no** pointer, length, count, or enum parameter, so null-pointer / zero-length / oversized-length / out-of-range-enum inputs are **unrepresentable** across this FFI boundary | not applicable — documented, nothing to test | `error_e15_no_pointer_or_enum_params` (documents the ABI) | [x] |

## Notes on E15 (the generic boundaries the prompt asks about)

The prompt requires covering null pointers, zero/oversized lengths, and
one-past-range enum values. For this API those categories are genuinely
unrepresentable, and that claim is checked mechanically rather than asserted:

* `driver`'s only parameter is a by-value `double` (`c_double` in Rust) — there
  is no pointer to nullify and no length to zero or oversize.
* There is no `enum` anywhere in `c_src/`, so there is no integer-with-no-valid-
  variant to smuggle across the boundary.

The nearest true analogue of "an integer with no valid variant" for a
by-value `double` **is** a non-canonical bit pattern — a signaling NaN or a NaN
with an arbitrary payload, which no arithmetic would normally produce. Those are
covered by rows **E6** and **E7**, and additionally by the fully randomized
64-bit-pattern sweep in Phase B (`CONFIGS.md` row C21), which feeds
`f64::from_bits(random_u64)` and therefore reaches every exponent/mantissa/sign
class, including all NaN encodings.
