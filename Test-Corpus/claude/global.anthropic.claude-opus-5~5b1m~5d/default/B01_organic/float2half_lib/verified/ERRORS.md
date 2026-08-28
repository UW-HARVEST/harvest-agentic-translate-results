# ERRORS.md — Error-surface table (Phase A / Phase C)

## How this table was derived

Mechanically, by grepping the **entire** C source (`c_src/src/lib.c`,
`c_src/include/lib.h` — 121 lines total) for every construct that could reject
input or signal an error. Match counts over both files, excluding the two
lookup-table literals:

| pattern searched | occurrences | notes |
|------------------|-------------|-------|
| `return`               | 1 | the single success `return` in `float2half` |
| `return -1` / `return NULL` / error enum / `RETURN_ERROR` | 0 | — |
| `assert`               | 0 | — |
| `errno`                | 0 | — |
| `error` (any case)     | 0 | — |
| `if` (statement)       | 0 | the 2 textual hits are the substring in `sh`**`if`**`t` |
| `else` / `switch` / `case` / `goto` | 0 | — |
| `for` / `while`        | 0 | — |
| `abort` / `exit`       | 0 | — |
| `#if` / `#ifdef` / `#ifndef` | 0 | no conditional compilation |
| `malloc` / `free`      | 0 | no allocation |
| `*` (pointer/deref)    | 0 | **no pointer appears anywhere in the API or body** |
| explicit range / bounds / min / max check | 0 | — |
| `enum`                 | 0 | — |

The complete non-table body of the library is:

```c
uint16_t float2half(float flt) {
    union { float flt; uint32_t num; } in;
    uint32_t n, j;
    in.flt = flt;
    n = in.num;
    j = (n >> 23) & 0x1ff;
    return (uint16_t)((uint32_t)m__base[j] + ((n & 0x007fffff) >> m__shift[j]));
}
```

## Result: the error surface is EMPTY — and that is itself the property to test

`float2half` is a **total, branchless function**. It has:

* no error return value (every one of the 2^16 `uint16_t` values is a legal
  result, so there is no sentinel available and none is used),
* no pointer parameters — so **null-pointer tests are not applicable**,
* no length/size/count parameters — so **zero-length and oversized-length
  tests are not applicable**,
* no `enum` parameters — so **out-of-range enum values are not applicable**
  (there is no enum to pass an invalid `int` for),
* no option/flag parameter — so there is no invalid-flag case,
* exactly one parameter, a `float`, and **every one of the 2^32 bit patterns
  of a `float` is a valid, accepted input** that returns normally.

Therefore Phase C cannot be "one test per rejection branch"; instead the
correct Phase C obligation is to prove that **neither implementation rejects,
traps, aborts or diverges on any input**, including all the inputs that would
be errors in a less total API. That is what the rows below assert. Each row
names a condition that is a *potential* failure mode of the implementation
(not a documented C rejection), states what the C provably does, and is
checked off only when a differential test confirms the Rust does the same.

The Rust translation is where a rejection could be *introduced* that the C
does not have — an out-of-bounds slice index or an arithmetic overflow in Rust
panics, and with `panic = "abort"` in `[profile.release]` that terminates the
process. Rows 1–4 exist specifically to prove no such Rust-only rejection
exists.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `float2half` | Table index out of range: any input whose top 9 bits reach the end of the tables, i.e. `j == 511` (`0xFFFFFFFF`, negative NaN, all mantissa bits set) and `j == 0` (`0x00000000`). C masks with `& 0x1ff`, so `j` is provably in `0..=511`; C never reads out of bounds and cannot fault. Rust indexes `[u16; 512]`/`[u8; 512]` with the same masked value. | Returns normally. `0xFFFFFFFF` -> `0xFFFF`; `0x00000000` -> `0x0000`. No trap. | `err_row01_index_domain_never_out_of_bounds` | [x] |
| 2 | `float2half` | Every one of the 512 possible table indices `j`, driven from the input side (all 256 exponents x both signs), to prove no index in the domain faults or diverges. | Returns normally for all 512; result equals `m__base[j] + ((mant & 0x7fffff) >> m__shift[j])`. | `err_row02_all_512_indices_reachable_and_safe` | [x] |
| 3 | `float2half` | Maximum shift amount: inputs with `m__shift[j] == 24` (`j` in `0..=102`, `143..=254`, `256..=358`, `399..=510`). Shifting a 32-bit value right by 24 is well-defined in C (24 < 32). A Rust `>>` by an amount `>=` bit width would panic in debug / be UB-free-but-wrong in release. | Returns normally; the shifted mantissa term is always exactly 0, so the result is exactly `m__base[j]`. | `err_row03_max_shift_amount_24` | [x] |
| 4 | `float2half` | Arithmetic overflow of the sum: worst case `m__base[j] + ((0x7fffff) >> m__shift[j])`. C computes in `uint32_t` then narrows with `(uint16_t)`. In Rust a plain `+` on `u16` would panic on overflow in debug builds. Mechanically checked over all 512 indices: the maximum attainable sum is exactly `65535` (`0xFFFF`, at `j == 511`), so the narrowing cast never actually truncates — but the sum must still be computed without a Rust overflow panic. | Returns normally; `0xFFFFFFFF` -> `0xFFFF` (the exact maximum, no wrap). | `err_row04_sum_never_overflows_u16` | [x] |
| 5 | `float2half` | Signalling NaN input (`0x7FA00000`, `0xFFA00000`) — an input that traps in FPU arithmetic. C only type-puns the bits through a `union`, it never performs an arithmetic operation on the float, so no FP exception is raised and the payload is read verbatim. The Rust must likewise not launder the value through an arithmetic op (which could quiet the NaN and change the payload bits). | Returns normally; sNaN payload bits are read verbatim: `0x7FA00000` -> `0x7C00 + (0x200000 >> 13)` = `0x7D00`. | `err_row05_signalling_nan_payload_not_quieted` | [x] |
| 6 | `float2half` | Quiet NaN whose payload is small enough to be shifted away (`0x7F800001`, mantissa `1`, `1 >> 13 == 0`). The C silently maps a NaN to `0x7C00`, i.e. **NaN becomes Infinity** — an "incorrect"-looking result that must be replicated, not fixed. | Returns `0x7C00` (`+Inf` in binary16) for a `+NaN` input; `0xFC00` for the `-NaN` input `0xFF800001`. | `err_row06_nan_degenerates_to_infinity` | [x] |
| 7 | `float2half` | Every NaN payload (all `2^23 - 1` non-zero mantissas at exponent 255, both signs) — the only value-dependent path at `j == 255`/`511`, where `shift == 13` rather than 24. A translation that special-cased NaN would diverge here. | Returns `0x7C00 + (mant >> 13)` / `0xFC00 + (mant >> 13)`, spanning `0x7C00..=0x7FFF` / `0xFC00..=0xFFFF`. | `err_row07_all_nan_payloads_both_signs` | [x] |
| 8 | `float2half` | Values one step past the range that is representable in binary16: the largest input that still maps to a finite half vs. the first that saturates (`j == 142` -> `j == 143`, where `shift` jumps 13 -> 24), and the overflow-to-`Inf` region generally. | Returns normally; `j == 143..=254` always yields exactly `0x7C00` regardless of mantissa (mantissa is shifted out by 24). | `err_row08_one_past_finite_half_range` | [x] |
| 9 | `float2half` | Values one step past the range that is representable at all in binary16, i.e. underflow: the largest input that still maps to a non-zero half subnormal vs. the first that flushes to zero (`j == 103` -> `j == 102`). | Returns normally; `j == 0..=102` always yields exactly `0x0000` (and `0x8000` for the negative mirror), i.e. **negative underflow returns `-0`, not `0`**. | `err_row09_one_past_representable_underflow` | [x] |
| 10 | `float2half` | Both zeros and both infinities — the degenerate special values (`+0`, `-0`, `+Inf`, `-Inf`). | `+0 -> 0x0000`, `-0 -> 0x8000`, `+Inf -> 0x7C00`, `-Inf -> 0xFC00`. | `err_row10_zeros_and_infinities` | [x] |
| 11 | `float2half` | Float subnormal *inputs* (exponent 0, non-zero mantissa) — inputs many conversion routines special-case. C does not special-case them: `j == 0`, so `base == 0` and `shift == 24`. | Returns exactly `0x0000` (`0x8000` for negative float subnormals) for every float subnormal. | `err_row11_float_subnormal_inputs` | [x] |
| 12 | `float2half` | The exhaustive statement of rows 1–11: **all 2^32 bit patterns**, to prove there exists no input at all on which either implementation rejects, aborts, or differs. | Returns normally for all 2^32 inputs; no input aborts. | `exhaustive_all_2_pow_32_bit_patterns` (in `phase_d_exhaustive.rs`) | [x] |

**Rows: 12. Unchecked rows: 0.**

## Supplementary tests (not tied to a single row)

Because the mechanical grep produced no rejection branches, the suite adds
these guards so the "empty error surface" claim is enforced rather than assumed:

| test | file | what it pins down |
|------|------|-------------------|
| `err_generic_boundaries_are_inapplicable_by_construction` | `phase_c_errors.rs` | Re-reads `c_src/include/lib.h` and asserts the public signature still has no pointer, no length/size/count parameter, no `enum` and no `struct`. If the header ever grows one, this test fails and the corresponding null-pointer / zero-length / invalid-enum rows become required. |
| `err_one_step_past_every_exponent_boundary` | `phase_c_errors.rs` | For all 256 exponents x both signs: the last mantissa of the exponent, the first of the next, and the raw bit-pattern neighbours either side — i.e. one step past every range endpoint the table has. |
| `exhaustive_special_regions_unstrided` | `phase_d_exhaustive.rs` | Unconditional full 2^23-mantissa sweep of the trickiest indices (exp 255 Inf/NaN, exp 103..112 varying-shift subnormals, exp 0/113/142/143 region edges), both signs — so even a strided CI run keeps exhaustive coverage where it matters. |
| `exhaustive_c_matches_table_model` | `phase_d_exhaustive.rs` | Over all 2^32 inputs, checks the C `.so` against a model built by parsing `m__base`/`m__shift` out of `c_src/src/lib.c`. Combined with the C-vs-Rust sweep this proves the Rust implements the C tables exactly. |
| `symbol_parity_c_so_vs_rust_so` | `phase_d_exhaustive.rs` | Runs `nm -D` on both `.so` files inside the test suite and fails if any C symbol is missing from the Rust `.so`, or if either library leaks `m__base`/`m__shift` (they have internal linkage in C). |

## Suite adequacy: mutation testing

An error table can be complete and the tests still be vacuous, so
`mutation_check.py` injects 10 deliberate bugs into `src/lib.rs`, rebuilds the
Rust `.so`, and requires the suite to FAIL on each one (the original source is
always restored). All 10 are caught:

| mutation | injected bug | caught by |
|----------|--------------|-----------|
| M1 | index mask `0x1ff` -> `0x0ff` (sign bit dropped) | 29 tests |
| M2 | `m__shift[255]` `13` -> `24` (the plausible "NaN fix") | 17 tests |
| M3 | `m__shift[511]` `13` -> `24` (negative NaN) | 16 tests |
| M4 | `m__base[300]` off by one (mid-table corruption) | 14 tests |
| M5 | `m__shift[103]` `23` -> `22` (varying-shift region) | 14 tests |
| M6 | mantissa mask `0x007fffff` -> `0x00ffffff` | 28 tests |
| M7 | round-to-nearest instead of C's truncation | 26 tests |
| M8 | preserve NaN-ness instead of letting it degenerate to Inf | 19 tests |
| M9 | `m__base[143]` no longer `Inf` (region edge) | 16 tests |
| M10 | `m__base[256]` `0x8000` -> `0x0000` (negative underflow loses `-0`) | 18 tests |

**10/10 mutations caught — no blind spots detected.**
