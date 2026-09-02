# ERRORS.md — Phase A error-surface table

## How this was derived

Mechanical grep of the entire C source (`c_src/src/lib.c`, 118 lines, and
`c_src/include/lib.h`, 3 lines) for every rejection construct:

```
grep -nE 'return -|return NULL|RETURN_ERROR|assert|errno|exit\(|abort\(|goto|if *\(|switch|#ifdef|#if |#else|enum |\?|<|>|==|!=' \
     c_src/src/lib.c c_src/include/lib.h
```

Matches, after excluding the 1024 table-literal lines: only `#include <stdint.h>`
(both files) and the two body lines 116–117. `grep -c return c_src/src/lib.c` = **1**.

## Result: the error surface is EMPTY

`float2half` is a **total, branch-free function**:

* it takes one `float` **by value** — there is no pointer parameter, so there is
  no null-pointer check and no null-pointer path to test;
* there is no length, count, size, or buffer parameter — so there is no zero
  length or oversized length path;
* there is no `enum` parameter anywhere in the public header — so there is no
  out-of-range-enum path (this class of bug cannot exist in this API);
* there are no `if`/`switch`/`?:`/`goto`/`#ifdef` statements, no `assert`, no
  `errno` use, no `return -1` / `return NULL` / error macro, and no error enum;
* the single `return` is unconditional.

Every one of the 2^32 possible incoming bit patterns is therefore an *accepted*
input that produces a defined `uint16_t`. There is no input the C rejects, so
there is no error code or sentinel to match. The rows below are consequently
**not** "C returns an error" rows; they are the *implicit* invariants the C
relies on instead of checking, plus the generic-boundary classes the task
requires, restated as "what the C actually does". Each row has a differential
test asserting Rust returns the **same value** the C returns (and, for the
invariant rows, that neither side traps/panics/UBs where the other does not).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `float2half` | NULL pointer argument — **impossible**: the only parameter is a by-value `float` (`include/lib.h`). No pointer crosses the FFI boundary. | N/A — no such input exists; nothing to reject | `err_01_no_pointer_parameter_documented` | [x] |
| 2 | `float2half` | Zero / oversized length argument — **impossible**: there is no length, size, or count parameter. | N/A — no such input exists | `err_02_no_length_parameter_documented` | [x] |
| 3 | `float2half` | Out-of-range enum value across the FFI boundary — **impossible**: no `enum` appears in the public header or the implementation. | N/A — no such input exists | `err_03_no_enum_parameter_documented` | [x] |
| 4 | `float2half` | Table index out of range: `j` used to index `m__base[512]` / `m__shift[512]`. C does **not** bounds-check; it relies on `j = (n >> 23) & 0x1ff` masking to 9 bits. Adversarial input: sweep all 512 reachable `j` values, i.e. every (sign, exponent) pair, incl. `j == 0` and `j == 511`. | No OOB read possible; returns `m__base[j] + ((n & 0x7fffff) >> m__shift[j])` for every `j` in `0..=511`. Never errors. | `err_04_all_512_table_indices_in_range` | [x] |
| 5 | `float2half` | Undefined/oversized shift count: `(n & 0x007fffff) >> m__shift[j]`. C does **not** check the shift amount; it relies on every `m__shift` entry being `< 32`. Adversarial input: the max shift entry `0x18` (24) and the min `0x0d` (13), with the largest possible mantissa `0x7fffff`. | Table max is `0x18` (24) < 32, so the shift is always defined. With shift 24 the mantissa term is always 0. Never errors. | `err_05_shift_amount_always_below_32` | [x] |
| 6 | `float2half` | Arithmetic overflow of the `uint32_t` sum, then narrowing to `uint16_t`. C does **not** check. Adversarial input: rows that maximise `m__base[j]` (`0xfc00`) together with the maximal mantissa term (`0x3ff`). | Max sum is `0xfc00 + 0x3ff = 0xffff`, so neither the `uint32_t` addition nor the `(uint16_t)` narrowing ever loses information. Never errors. | `err_06_sum_never_overflows_u16` | [x] |
| 7 | `float2half` | Value one step past the representable range of the half destination: smallest float that overflows binary16 (`> 65504`), and the largest float below it. C does **not** range-check the value. | Saturates to `0x7c00` / `0xfc00` (or the last finite code) purely via the tables — no error, no errno, no clamp branch. | `err_07_past_half_range_no_rejection` | [x] |
| 8 | `float2half` | Values one step past the binary16 subnormal floor (underflow to zero). C does **not** check. | Returns `0x0000` / `0x8000` purely via the tables — no error. | `err_08_underflow_no_rejection` | [x] |
| 9 | `float2half` | Non-finite / non-numeric inputs: `+inf`, `-inf`, quiet NaN, **signalling** NaN (all NaN payloads incl. `0x7f800001` and `0xffffffff`). C does **not** special-case them and performs no FP arithmetic (it type-puns through a union), so no FP exception is raised and no canonicalisation happens. | Table lookup only; sNaN payload bits are preserved into the result the same way qNaN's are. Never errors. | `err_09_nan_inf_snan_bit_exact` | [x] |
| 10 | `float2half` | Negative zero and the two zero encodings. C does not special-case. | `0x0000` for `+0.0`, `0x8000` for `-0.0`. Never errors. | `err_10_signed_zeros` | [x] |
| 11 | `float2half` | Every remaining bit pattern, i.e. "garbage" reinterpreted as `float` — the closest analogue to an out-of-domain value for this API, since a `float` parameter accepts any 32-bit pattern. | Defined `uint16_t` for all 2^32 patterns; no rejection path exists. | `exhaustive_all_2_pow_32_bit_patterns` | [x] |

All 11 rows checked: see `tests/error_paths.rs` (rows 1–10) and
`tests/exhaustive.rs` (row 11).
