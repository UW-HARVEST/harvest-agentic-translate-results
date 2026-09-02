# ERRORS.md — error-surface table (Phase C gate)

Derived mechanically from `c_src/src/pow.c`. The complete set of
rejection/error constructs in the C source is:

```
$ grep -n 'return\|assert\|errno\|NULL\|if \|else' c_src/src/pow.c
33:  errno = 0;                       <- error state reset (pre-condition)
35:  if (errno == EDOM) {             <- rejection branch 1
39:    return -1;                     <-   sentinel
40:  } else if (errno == ERANGE) {    <- rejection branch 2
44:    return -1;                     <-   sentinel
47:  return result;                   <- success path
```

Facts that constrain the table:

* There are **exactly two** error-return statements, both `return -1`
  (`-1.0` as a `double`), each preceded by one `fprintf` to `stderr`.
* There are **no** `assert`s, **no** `NULL`/pointer checks, **no** explicit
  range checks, and **no** min/max constants — `my_pow` takes two `double`s
  by value and no pointers, sizes, counts or enums. So the classic
  null-pointer / zero-length / oversized-length / out-of-range-enum boundary
  classes have no reachable representation in this API. The equivalent
  "every representable input is passed" coverage is achieved instead by
  fuzzing the full 64-bit `double` bit space (including all NaN payloads,
  both infinities, both zeros and subnormals) — rows 12–15.
* `errno` is the *only* thing the C branches on, so the trigger for each row
  is "an input pair for which glibc's `pow` sets that `errno`". Rows 1–8
  enumerate the distinct ways glibc reaches `EDOM` / `ERANGE`; verified
  against glibc directly.
* Both error paths also emit a message. "Same rejection" is asserted as
  **return value AND the exact stderr bytes**, not merely "both failed".

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `my_pow` | domain error: finite `base < 0` with finite non-integral `exponent` (e.g. `-2, 0.5`) → `pow` returns `-nan`, sets `EDOM` | returns `-1.0`; stderr `Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n` |
| 2  | `my_pow` | domain error, sub-case: `base` in `(-1, 0)` with non-integral exponent (e.g. `-0.5, 0.5`) — same branch, different magnitude/formatting | returns `-1.0`; `Domain error: pow(-0.50, 0.50) …` |
| 3  | `my_pow` | pole error: `base == +0.0`, `exponent < 0` odd integer (`0, -1`) → `+inf`, `ERANGE` | returns `-1.0`; stderr `Range error: pow(0.00, -1.00) caused overflow or underflow.\n` |
| 4  | `my_pow` | pole error: `base == -0.0`, `exponent < 0` odd integer (`-0.0, -3`) → `-inf`, `ERANGE` | returns `-1.0`; `Range error: pow(-0.00, -3.00) …` |
| 5  | `my_pow` | pole error: `base == ±0.0`, `exponent < 0` even integer (`-0.0, -2`) → `+inf`, `ERANGE` | returns `-1.0`; `Range error: pow(-0.00, -2.00) …` |
| 6  | `my_pow` | pole error: `base == ±0.0`, `exponent < 0` non-integral (`0, -0.5`) → `+inf`, `ERANGE` | returns `-1.0`; `Range error: pow(0.00, -0.50) …` |
| 7  | `my_pow` | overflow: result magnitude `> DBL_MAX` (`10, 400`; also `-2, 1e300` → `+inf`) | returns `-1.0`; `Range error: pow(10.00, 400.00) …` |
| 8  | `my_pow` | underflow: result rounds to `0` / below the subnormal range (`10, -400`; `1e-300, 10`) | returns `-1.0`; `Range error: pow(10.00, -400.00) …` |
| 9  | `my_pow` | ordering: an input that *could* be read as both (negative base, non-integral exponent **and** overflow magnitude) must take the `EDOM` branch first because `if (errno == EDOM)` is tested before `ERANGE` | `EDOM` message, `-1.0` |
| 10 | `my_pow` | `errno` already set to `EDOM`/`ERANGE`/garbage **before** the call, with valid inputs: `errno = 0` at line 33 must clear it | no message; returns the true `pow` result (no spurious `-1.0`) |
| 11 | `my_pow` | `errno` left set by a *previous* erroring call, then a valid call: must not leak | no message; true result |
| 12 | `my_pow` | NaN inputs, which are *not* errors for glibc `pow` (`NaN, 2` → `NaN` errno 0; `NaN, 0` → `1`; `1, NaN` → `1`) — must **not** be rejected | no message; `NaN` / `1.0` respectively, never `-1.0` |
| 13 | `my_pow` | signalling-NaN and non-canonical NaN payloads in either argument | same as C bit-for-bit, no message |
| 14 | `my_pow` | `±inf` in either or both arguments (`inf,2`, `-inf,3`, `2,inf`, `-1,inf`, `-2,inf`) — not errors | no message; C's value bit-for-bit |
| 15 | `my_pow` | ambiguity of the sentinel: `pow(-1, 1) == -1.0` with `errno == 0` — a *successful* `-1.0` that must be returned unchanged (the C does not distinguish it from the error sentinel) | returns `-1.0`, **no** stderr output |
| 16 | `my_pow` | subnormal *result* that does **not** set `ERANGE` in glibc (`2, -1070` → `7.9e-323`, errno 0) — must not be rejected | no message; subnormal value |
| 17 | `my_pow` | `%.2f` formatting of the error message for pathological values: `-0.0` → `-0.00`, `NaN` → `nan`/`-nan`, `±inf` → `inf`/`-inf`, `1e300` → 309-digit expansion, half-way rounding (`0.005` → `0.01`) | stderr bytes identical to C |

## Status

| # | test | status |
|---|------|--------|
| 1 | `err_01_domain_negative_base_fractional_exponent` | [x] |
| 2 | `err_02_domain_negative_fraction_base` | [x] |
| 3 | `err_03_pole_pos_zero_negative_odd_int` | [x] |
| 4 | `err_04_pole_neg_zero_negative_odd_int` | [x] |
| 5 | `err_05_pole_zero_negative_even_int` | [x] |
| 6 | `err_06_pole_zero_negative_fractional` | [x] |
| 7 | `err_07_overflow` | [x] |
| 8 | `err_08_underflow` | [x] |
| 9 | `err_09_edom_checked_before_erange` | [x] |
| 10 | `err_10_preexisting_errno_cleared` | [x] |
| 11 | `err_11_errno_not_leaked_across_calls` | [x] |
| 12 | `err_12_nan_inputs_are_not_errors` | [x] |
| 13 | `err_13_nan_payloads` | [x] |
| 14 | `err_14_infinities` | [x] |
| 15 | `err_15_successful_minus_one_is_not_an_error` | [x] |
| 16 | `err_16_subnormal_result_no_erange` | [x] |
| 17 | `err_17_message_formatting_pathological_values` | [x] |

## Test sensitivity (negative control)

Passing tests only prove something if they can fail. `./mutation_check.sh`
builds 13 deliberately-broken copies of `src/pow.rs` and runs the *unmodified*
suite against each; all 13 are detected:

`no_errno_reset`, `skip_edom_branch`, `skip_erange_branch`, `return_plus_one`,
`swap_edom_erange_order`, `wrong_edom_value`, `wrong_erange_value`,
`errno_read_before_pow`, `return_result_on_edom`, `typo_in_message`,
`swap_printed_args`, `wrong_precision`, `no_trailing_newline`.

One further mutant was tried and *survives*, correctly: replacing
`unsafe { ffi::pow(base, exponent) }` with `base.powf(exponent)`. On this
toolchain `f64::powf` still lowers to a call to glibc's `pow` (the mutant's
`.so` still imports `pow@GLIBC_2.29`), so `errno` is still set and the mutant
is behaviourally equivalent. It is *not* a test gap. The explicit `ffi::pow`
declaration is kept anyway, because `llvm.pow.f64` is only permitted — not
required — to lower to the libm call, so relying on it would be relying on an
unguaranteed optimiser detail.
