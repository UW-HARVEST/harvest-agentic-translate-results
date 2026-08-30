# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source, not from docs or assumptions.

## Mechanical grep for rejection constructs

Every rejection idiom was grepped for across the entire C library
(`src/driver.c`, `include/driver.h`), excluding the licence comment block:

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|exit|abort|if|else|switch|case|while|for|\?|#ifdef|#if |goto|<|>|==|!=|&&|\|\|' src/driver.c include/driver.h
src/driver.c:26:#include <stdint.h>
src/driver.c:27:#include <stdio.h>
include/driver.h:24:#ifndef DRIVER_H_
include/driver.h:29:#endif //DRIVER_H_
```

The only matches are two `#include` lines and the header's own include guard.
Concretely, the C library contains:

* **0** `return` statements (`driver` returns `void`)
* **0** error-return macros / sentinel returns (`return -1`, `return NULL`, …)
* **0** `assert` / `abort` / `exit` calls
* **0** error enums or status codes — the only entry point's return type is `void`
* **0** `if` / `else` / `switch` / `?:` / loop branches
* **0** range checks, null checks, or min/max constants
* **0** pointer parameters anywhere in the API (the sole parameter is a `double`
  passed by value), hence no null-pointer rejection is even expressible
* **0** enum parameters, hence no out-of-range-enum path is expressible
* **0** length/size/count parameters, hence no zero-length or oversized-length
  path is expressible

## The error surface is EMPTY — and that is a finding, not an omission

`void driver(double f)` is a **total function over its entire input domain**. Its
parameter is a single `double` passed by value, so *every one of the 2^64
possible bit patterns is a valid input*. There is no value of `f` that the C code
rejects, no value that makes it return an error (it cannot: it returns `void`),
and no value that makes it abort. It unconditionally executes one `printf` and
returns.

Therefore the error-surface table has **no rows for input rejection**. Writing
invented rows here would be fiction. Instead, the table below enumerates the
generic C-API boundaries the task calls out, and records, for each, what the C
*actually* does — which in every case is "accepts it and prints", i.e. a
**non-rejection**. Each row still gets a differential test: the assertion is that
C and Rust agree on the *same non-rejection* (byte-identical output, no crash,
no trap), which is the correct analogue of "same error code" for a total
function. These are precisely the inputs that a happy-path test would miss.

| # | function | trigger (the exact invalid/boundary input or condition) | expected C result | test |
|---|----------|---------------------------------------------------------|-------------------|------|
| E1 | `driver` | Null pointer argument — **not expressible**: the sole parameter is `double` by value, there is no pointer in the ABI. Nearest analogue: the all-zero-bits argument, `f = +0.0`. | No rejection. Prints `0 0x0p+0 0.0000`. Returns `void`. | `err_e1_all_zero_bits_no_pointer_to_be_null` |
| E2 | `driver` | Zero length — **not expressible**: no length/size/count parameter exists. Nearest analogue: `f = -0.0` (sign bit set, zero magnitude), the boundary value that distinguishes `%llx` from `%.4f` output. | No rejection. Prints `8000000000000000 -0x0p+0 -0.0000`. | `err_e2_negative_zero` |
| E3 | `driver` | Oversized length — **not expressible**: no length parameter. Nearest analogue: the largest finite magnitude, `f = ±DBL_MAX` (`0x7fefffffffffffff` / `0xffefffffffffffff`), which makes `%.4f` emit a ~310-character digit string — the longest output the API can produce. | No rejection. Prints the full ~310-digit expansion; no truncation, no overflow. | `err_e3_dbl_max_longest_output` |
| E4 | `driver` | One step past the valid range, high end: `f = nextafter(DBL_MAX, INF) = +INFINITY` (`0x7ff0000000000000`). The exponent field leaves the finite range. | No rejection. Prints `7ff0000000000000 inf inf`. | `err_e4_positive_infinity` |
| E5 | `driver` | One step past the valid range, low end: `f = -INFINITY` (`0xfff0000000000000`). | No rejection. Prints `fff0000000000000 -inf -inf`. | `err_e5_negative_infinity` |
| E6 | `driver` | Not-a-number, positive quiet NaN (`0x7ff8000000000000`) — a value with no meaningful numeric interpretation, the float analogue of "an enum value with no valid variant". | No rejection. Prints `7ff8000000000000 nan nan`. | `err_e6_quiet_nan_positive` |
| E7 | `driver` | Negative quiet NaN (`0xfff8000000000000`) — sign bit set on a NaN; glibc spells this `-nan`, so the sign must survive the FFI boundary. | No rejection. Prints `fff8000000000000 -nan -nan`. | `err_e7_quiet_nan_negative` |
| E8 | `driver` | **Signalling** NaN (`0x7ff0000000000001`) — mantissa MSB clear. Passing this across the FFI boundary must not quiet it (which would corrupt the `%llx` bit pattern) nor raise an invalid-operation trap. | No rejection. `%llx` prints the sNaN bits unmodified; `%a`/`%.4f` print `nan`. | `err_e8_signalling_nan` |
| E9 | `driver` | NaN carrying an arbitrary payload (e.g. `0x7ff8deadbeefcafe`, `0xfffdeadbeefcafe`-class patterns) — the payload bits must be preserved verbatim by `%llx` even though `%a` collapses them all to `nan`. | No rejection. Payload appears in `%llx`; `%a`/`%.4f` print `nan`/`-nan`. | `err_e9_nan_payload_preserved` |
| E10 | `driver` | Subnormal boundary: the smallest positive denormal `f = 5e-324` (`0x0000000000000001`), where the implicit mantissa bit is absent and glibc must switch to the `0x0.…p-1022` form for `%a`. | No rejection. Prints `1 0x0.0000000000001p-1022 0.0000`. | `err_e10_smallest_subnormal` |
| E11 | `driver` | The normal/subnormal transition, one step *below* the smallest normal: `f = nextafter(DBL_MIN, 0) = 0x000fffffffffffff` (largest subnormal), versus `DBL_MIN = 0x0010000000000000`. Crossing this boundary changes the `%a` mantissa form. | No rejection. Both print their respective `%a` forms; formats differ across the boundary. | `err_e11_subnormal_normal_boundary` |
| E12 | `driver` | `%.4f` underflow to zero with sign retention: a tiny negative value such as `f = -1e-300` rounds to `-0.0000`, not `0.0000`. A sign-dropping bug is invisible for positive inputs. | No rejection. Prints `-0.0000` for the `%.4f` field. | `err_e12_tiny_negative_signed_zero` |
| E13 | `driver` | `%.4f` round-half-to-even ties, resolved off the *exact binary* value, not the decimal literal: e.g. `0.00005`, `0.00015`, `0.00025`, `2.5e-5`. Naive round-half-away implementations diverge here. | No rejection. glibc rounds using the exact binary expansion; ties go to even only when the binary value is an exact tie. | `err_e13_round_half_even_ties` |
| E14 | `driver` | Every remaining bit pattern, including the ones no source-level `double` literal can name: swept by feeding raw `u64` bit patterns straight through the ABI (all exponent fields, both signs, random mantissas). No pattern is rejected by C. | No rejection for any of the 2^64 patterns; each produces exactly one line of output. | `err_e14_raw_bit_pattern_sweep` |

## Notes on rows E1–E3

Rows E1, E2 and E3 are the task's mandated "null pointer / zero length /
oversized length" boundaries. This API has **no pointer and no length parameter**,
so those conditions cannot be constructed — the ABI provides no way to express
them. Rather than skip the rows, each is mapped to the closest structurally
analogous boundary in the domain that *does* exist (all-zero bits, signed zero,
and maximum-magnitude/longest-output respectively) and tested there. This is
recorded explicitly so the mapping is auditable rather than silently dropped.

## Note on row E8 (signalling NaN)

E8 is the row that most resembles "an out-of-range enum value crossing the FFI
boundary". A signalling NaN is a bit pattern the type system admits but that no
arithmetic operation would produce, and a translation that let the value be
loaded/stored through an x87 register, or that round-tripped it through any
arithmetic, would silently quiet it (setting the mantissa MSB) and change the
`%llx` output. The C never inspects the value, so the Rust must not either.
