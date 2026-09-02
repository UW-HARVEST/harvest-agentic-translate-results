# ERRORS.md — error-surface table

## How this table was derived

Mechanical grep of the entire C source for every rejection construct:

```sh
grep -nE 'return|assert|NULL|errno|ERROR|if *\(|switch|exit|abort|-1' \
     c_src/src/driver.c c_src/include/driver.h        # -> 0 hits in code
```

Result: the C code contains

* **no** `return` statement (the function is `void`),
* **no** `assert`,
* **no** `if` / `switch` / ternary — i.e. no explicit range, null or bounds check,
* **no** error enum, error macro, `errno` use, sentinel value, `exit` or `abort`,
* **no** min/max constant,
* **no** pointer, array, length, count or enum parameter (the only parameter is
  a by-value `double`),
* the return value of `printf` (the one call that *can* fail, e.g. `EBADF`) is
  **discarded**, so even a failing write is invisible to the caller.

Consequently the library has an **empty explicit error surface**: there is no
input the C code rejects, and no observable error channel. `driver` accepts all
2^64 bit patterns of its argument and always returns `void`.

The rows below therefore enumerate the *implicit* rejection/degenerate surface
that actually exists for this API: the argument classes that the C library must
handle without a valid numeric representation (the `double` analogue of
"out-of-range enum value"), plus the generic FFI boundary conditions the task
mandates. Each row is asserted by a differential test in
`tests/differential.rs` (`phase_c_*`), comparing C vs Rust byte-for-byte.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `driver` | `+inf` (`0x7ff0000000000000`) — no finite value; `%a` and `%.4f` have no digits to print | writes `7ff0000000000000 inf inf\n`; returns normally, no error signalled | `phase_c_row01_pos_inf` |
| 2 | `driver` | `-inf` (`0xfff0000000000000`) — sign bit set on a non-finite | writes `fff0000000000000 -inf -inf\n` | `phase_c_row02_neg_inf` |
| 3 | `driver` | quiet NaN, positive sign (`0x7ff8000000000000`) — the "no valid variant" bit pattern | writes `7ff8000000000000 nan nan\n` | `phase_c_row03_qnan_pos` |
| 4 | `driver` | quiet NaN, **sign bit set** (`0xfff8000000000000`) — `%a`/`%.4f` must still honour `signbit` | writes `fff8000000000000 -nan -nan\n` | `phase_c_row04_qnan_neg` |
| 5 | `driver` | signalling NaN (`0x7ff0000000000001`, mantissa MSB clear) — must not trap or normalise | writes `7ff0000000000001 nan nan\n` (payload not printed) | `phase_c_row05_snan` |
| 6 | `driver` | NaN with arbitrary/maximal payload (`0x7fffffffffffffff`, `0xffffffffffffffff`) — payload must be dropped by `%a` yet preserved by `%llx` | `%llx` prints the full payload, `%a`/`%.4f` print `nan`/`-nan` | `phase_c_row06_nan_payloads` |
| 7 | `driver` | negative zero (`0x8000000000000000`) — zero that must keep its sign | writes `8000000000000000 -0x0p+0 -0.0000\n` | `phase_c_row07_negative_zero` |
| 8 | `driver` | positive zero (`0x0000000000000000`) — degenerate `%llx` (leading-zero suppression must yield a single `0`, not an empty field) | writes `0 0x0p+0 0.0000\n` | `phase_c_row08_positive_zero` |
| 9 | `driver` | smallest subnormal (`0x0000000000000001`) — biased exponent field 0, so `%a` must **not** renormalise | writes `1 0x0.0000000000001p-1022 0.0000\n` | `phase_c_row09_min_subnormal` |
| 10 | `driver` | largest subnormal (`0x000fffffffffffff`) — one step below the smallest normal | `%a` uses leading `0` and `p-1022` | `phase_c_row10_max_subnormal` |
| 11 | `driver` | smallest normal (`0x0010000000000000`) — one step past the subnormal range | `%a` uses leading `1` and `p-1022` | `phase_c_row11_min_normal` |
| 12 | `driver` | largest finite `DBL_MAX` (`0x7fefffffffffffff`) — one step below `inf`; `%.4f` must emit the full 309-digit exact expansion | full exact decimal expansion, no truncation/rounding to `inf` | `phase_c_row12_dbl_max` |
| 13 | `driver` | `-DBL_MAX` (`0xffefffffffffffff`) | same as row 12 with a leading `-` | `phase_c_row13_neg_dbl_max` |
| 14 | `driver` | magnitude strictly below the `%.4f` resolution (e.g. `4.9e-324`, `1e-300`, `1e-5`) — every requested fractional digit is a rounding artefact | `%.4f` collapses to `0.0000` / `-0.0000`, sign preserved | `phase_c_row14_underflow_to_zero` |
| 15 | `driver` | exact decimal tie at the 4th fractional digit (e.g. `0.03125`, `0.09375`, `2.00005`-class values) — round-half-to-even boundary, the "one step past valid range" of the rounding rule | glibc rounds half-to-even under the default `FE_TONEAREST` | `phase_c_row15_ties_half_even` |
| 16 | `driver` | rounding carry that propagates across the radix point (e.g. `0.99999`, `9.99999`, `-0.99999`) | `%.4f` produces `1.0000` / `10.0000` / `-1.0000` | `phase_c_row16_rounding_carry` |
| 17 | `driver` | every one of the 2^11 raw biased-exponent field values, including the reserved `0` and `0x7ff` — the exhaustive "out-of-range enum value" sweep for the only enumerable field in the input | C handles all; no field value is rejected | `phase_c_row17_all_exponent_fields` |
| 18 | `driver` | the argument is passed by value, so there is **no** null-pointer, zero-length or oversized-length input to construct; documented here for completeness | not applicable — no pointer/length parameter exists in the ABI | `phase_c_row18_no_pointer_or_length_surface` (asserts the ABI/arity of the export instead) |
| 19 | `driver` | every sentinel input (`±inf`, quiet/signalling NaN, `±0.0`, min/max subnormal, min normal) under **every** locale and **every** rounding direction — the degenerate inputs must not acquire a radix character where C prints none, and must not be nudged by directed rounding | identical sentinel spellings in all 4 × 8 ambient states | `phase_c_row19_specials_under_all_ambient_state` |
| 20 | `driver` | an out-of-range rounding-direction value — `fesetround` rejects any int that names no `FE_*` direction, so the reachable set is exactly the four modes; the translation's int→mode mapping must be total and must not silently collapse to the default | all four modes produce distinct, matching output; a bogus mode is refused by libc before `driver` ever sees it | `phase_c_row20_out_of_range_rounding_mode_value` |

All rows 1–17 and 19–20 are covered by `tests/differential.rs`; row 18 is
structurally impossible for this ABI and is recorded so the enumeration is
complete (the test in its place pins the export's ABI and arity).
