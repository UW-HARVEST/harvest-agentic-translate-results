# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/pow.c` (the whole file is 50 lines; every
statement was inspected). Grep evidence:

```sh
$ grep -nE 'return|assert|errno|ERROR|NULL|if *\(' c_src/src/pow.c
25:#include <errno.h>
31:double my_pow(double base, double exponent) {
33:  errno = 0;
35:  if (errno == EDOM) {
40:    return -1;
41:  } else if (errno == ERANGE) {
45:    return -1;
46:  }
48:  return result;
```

There are **no** `assert`s, **no** `RETURN_ERROR`-style macros, **no** error
enums, **no** pointer parameters (hence no null checks), and **no** explicit
range/min/max constants in the C source. The complete rejection surface is the
two `errno` branches, whose triggers are the conditions under which glibc's
`pow` sets `EDOM` / `ERANGE`. Each distinct *trigger* gets its own row because
each is a separately constructible invalid input.

`expected C result` below always means: the exact `double` bit pattern returned
**and** the exact bytes written to `stderr`. Both are compared against Rust.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| E1 | `my_pow` | `errno == EDOM` branch (line 35): negative finite `base`, finite non-integer `exponent`. e.g. `my_pow(-2.0, 0.5)` | returns `-1.0`; writes `Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n` to `stderr` |
| E2 | `my_pow` | same EDOM branch, non-integer exponent whose `%.2f` rendering *looks* integral (formatting edge). e.g. `my_pow(-3.0, 2.001)` | returns `-1.0`; domain-error line with `%.2f` = `2.00` |
| E3 | `my_pow` | same EDOM branch, negative base very close to `-0.0` (subnormal). e.g. `my_pow(-5e-324, 0.5)` | returns `-1.0`; domain-error line |
| E4 | `my_pow` | same EDOM branch, huge negative base. e.g. `my_pow(-1.7976931348623157e308, 1.5)` | returns `-1.0`; domain-error line |
| E5 | `my_pow` | `errno == ERANGE` branch (line 41) — **pole error**: `base == +0.0`, `exponent < 0` (odd integer). e.g. `my_pow(0.0, -1.0)` | returns `-1.0`; writes `Range error: pow(0.00, -1.00) caused overflow or underflow.\n` |
| E6 | `my_pow` | pole error: `base == -0.0`, `exponent < 0` odd integer. e.g. `my_pow(-0.0, -3.0)` | returns `-1.0`; range-error line, `%.2f` of `-0.0` is `-0.00` |
| E7 | `my_pow` | pole error: `base == ±0.0`, `exponent < 0` even integer / non-integer. e.g. `my_pow(0.0, -2.0)`, `my_pow(-0.0, -0.5)` | returns `-1.0`; range-error line |
| E8 | `my_pow` | ERANGE — **overflow**: finite operands, `\|result\| > DBL_MAX`. e.g. `my_pow(1e300, 2.0)`, `my_pow(10.0, 400.0)` | returns `-1.0`; range-error line |
| E9 | `my_pow` | ERANGE — overflow with negative base and odd integer exponent (result `-inf`). e.g. `my_pow(-1e300, 3.0)` | returns `-1.0`; range-error line |
| E10 | `my_pow` | ERANGE — **underflow to zero**: e.g. `my_pow(1e-300, 2.0)`, `my_pow(10.0, -400.0)` | returns `-1.0`; range-error line |
| E11 | `my_pow` | ERANGE — underflow into the **subnormal** range (may or may not raise on this glibc; whatever C does, Rust must match). e.g. `my_pow(2.0, -1070.0)` | returns `-1.0` **iff** glibc sets `ERANGE`, else the subnormal result — differential test asserts equality either way |
| E12 | `my_pow` | ERANGE — overflow just past the boundary: `my_pow(DBL_MAX, 1.0000000001)` | returns `-1.0`; range-error line |
| E13 | `my_pow` | branch-precedence check: an input where `EDOM` is set — must take the *first* branch, never the `ERANGE` one (verifies `if`/`else if` order at lines 35/41) | domain-error text, not range-error text |
| E14 | `my_pow` | `errno` left non-zero by a *previous* failing call, then a **valid** call: line 33 (`errno = 0`) must clear it so no error is reported. e.g. `my_pow(-2.0,0.5)` then `my_pow(2.0,3.0)` | second call returns `8.0`, writes nothing |
| E15 | `my_pow` | `errno` pre-set to `EDOM`/`ERANGE` by the caller before entry, then a valid call | returns the real result, writes nothing (line 33 clears) |

## Generic FFI boundary cases (required even though absent from the table)

The C signature is `double my_pow(double, double)`: there are **no pointers,
no lengths, no enums and no arrays**, so the classic null-pointer /
zero-length / oversized-length / out-of-range-enum cases have no
representation in this ABI. The equivalent "one step past valid range" and
"garbage value across the FFI boundary" inputs for a `double` parameter are
covered instead:

| #  | trigger | expected |
|----|---------|----------|
| G1 | `NaN` (quiet, default payload) as `base`, as `exponent`, as both | bit-identical return; identical `stderr` |
| G2 | signalling-NaN bit pattern `0x7FF0000000000001` passed as `base` / `exponent` | bit-identical return |
| G3 | `NaN` with a non-zero payload `0x7FF8DEADBEEFCAFE` (NaN payload propagation) | bit-identical return |
| G4 | `±INFINITY` as `base` and/or `exponent` (all 8 sign combinations) | bit-identical return |
| G5 | `±0.0` (both signs of zero) as `base` and/or `exponent` | bit-identical return; `-0.00` vs `0.00` in any message |
| G6 | `±DBL_MAX`, `±DBL_MIN`, `±5e-324` (subnormal min) | bit-identical return |
| G7 | exponent one step past the largest exactly-representable odd integer (`2^53±1`) | bit-identical return |
| G8 | fully random 64-bit patterns reinterpreted as `double` (includes every exponent/NaN/subnormal class) | bit-identical return |

## Phase C status — all rows pass

| row | test in `tests/phase_c_error_paths.rs` | [x] |
|-----|----------------------------------------|-----|
| E1  | `e1_edom_negative_base_non_integer_exponent` | [x] |
| E2  | `e2_edom_non_integer_exponent_that_prints_as_integer` | [x] |
| E3  | `e3_edom_subnormal_negative_base` | [x] |
| E4  | `e4_edom_huge_negative_base` | [x] |
| E5  | `e5_erange_pole_positive_zero_negative_odd_exponent` | [x] |
| E6  | `e6_erange_pole_negative_zero_negative_odd_exponent` | [x] |
| E7  | `e7_erange_pole_even_and_non_integer_exponents` | [x] |
| E8  | `e8_erange_overflow_positive` | [x] |
| E9  | `e9_erange_overflow_negative_base_odd_exponent` | [x] |
| E10 | `e10_erange_underflow_to_zero` | [x] |
| E11 | `e11_underflow_into_subnormal_range` | [x] |
| E12 | `e12_erange_overflow_just_past_boundary` (plus the just-below companions, which must **not** error) | [x] |
| E13 | `e13_edom_branch_takes_precedence_over_erange` | [x] |
| E14 | `e14_errno_cleared_between_calls` | [x] |
| E15 | `e15_caller_preset_errno_does_not_leak_into_the_result` | [x] |
| G1  | `g1_quiet_nan_operands` | [x] |
| G2  | `g2_signalling_nan_bit_patterns` | [x] |
| G3  | `g3_nan_payload_propagation` | [x] |
| G4  | `g4_all_infinity_sign_combinations` | [x] |
| G5  | `g5_both_signs_of_zero` | [x] |
| G6  | `g6_extreme_finite_magnitudes` | [x] |
| G7  | `g7_exponent_one_step_past_exact_integer_range` | [x] |
| G8  | `g8_random_raw_bit_patterns` (3 600 random pairs, fixed seed) | [x] |

```
$ cargo test --release --test phase_c_error_paths
test result: ok. 23 passed; 0 failed
```

### The assertions are specific, not "both failed somehow"

Rows E1–E10 go through `diff_expect_rejection`, which asserts, beyond C/Rust
equality, that the shared behaviour really is the documented rejection:

* the returned value is exactly `-1.0` (the C sentinel), and
* the `stderr` text starts with `Domain error: pow(` or `Range error: pow(` as
  the row requires, and
* `errno` afterwards is `EDOM` or `ERANGE`.

E13 additionally asserts the *negative*: for inputs that satisfy both error
conditions the diagnostic must be the domain-error one and must **not** contain
`Range error`, pinning the `if` / `else if` order at lines 35/41.

E11 and E12 are deliberately equality-only where glibc's choice is
implementation-defined (whether a gradual underflow to a non-zero subnormal
raises `ERANGE`). Asserting a specific outcome there would encode my guess about
glibc rather than the C code's behaviour; the differential assertion is what
matters and it holds either way.

### Note on the unrepresentable generic cases

The task's generic checklist calls for null pointers, zero/oversized lengths and
out-of-range enum values. `my_pow`'s ABI is `double(double, double)` — it has no
pointer, length, array or enum parameter, so those inputs cannot be expressed at
this boundary. The faithful analogue for a `double` parameter is *every bit
pattern the type admits, including those with no valid numeric meaning*: G1–G8
cover quiet NaNs, signalling NaNs, non-canonical NaN payloads, both infinities,
both zeros, the subnormal extremes, and 3 600 fully random 64-bit patterns.
`tests/phase_d_sweeps.rs::d13`/`d14` extend this to a systematic walk over all
2048 biased-exponent fields of each parameter, and the `errno`-valued inputs
(`E15`, `C34`) include `i32::MIN`, `i32::MAX` and `-1`, which are the
out-of-range values for the one enum-like quantity that does cross this
boundary.
