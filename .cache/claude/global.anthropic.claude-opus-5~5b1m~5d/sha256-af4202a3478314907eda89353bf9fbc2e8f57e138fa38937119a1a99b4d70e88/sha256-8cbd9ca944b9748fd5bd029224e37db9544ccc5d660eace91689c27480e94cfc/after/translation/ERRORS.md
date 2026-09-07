# ERRORS.md — Error-surface table

## Mechanical derivation

Every rejection/error construct was grepped for across the *entire* C source
(`c_src/src/lib.c`, 32 lines; `c_src/include/lib.h`, 1 line — these are the only
C files, per `CMakeLists.txt`):

```
$ grep -nE 'return|assert|NULL|errno|-1|abort|exit|if *\(|\?|enum|#define|#if' src/lib.c include/lib.h
src/lib.c:8:        if (src[0] < src[1]) {
src/lib.c:15:  0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
src/lib.c:25:  0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
```

Findings:

* **0** `return` statements (the function is `void`; it falls off the end).
* **0** `RETURN_ERROR`-style macros, **0** error enums, **0** error codes,
  **0** sentinel returns — there is no channel to report an error on.
* **0** `assert` / `abort` / `exit` calls (`<assert.h>` is not even included).
* **0** `NULL` checks on `dest` or `src`.
* **0** explicit length/range/size validation of `count`.
* **0** min/max named constants, **0** `#define`s, **0** `#if`/`#ifdef`.
* No `enum` parameters anywhere in the API, so there is no out-of-range-enum
  class of input to test (`count` is a plain `int` and every `int` value is
  valid input to the loop guard).
* The only value-conditioning in the whole library is the
  `(((0) > (sqd)) ? (0) : (sqd))` clamp — an internal *saturation*, not a
  rejection; it never reports anything to the caller.

So the C library's *explicit* error surface is empty: `tfm` accepts every
argument combination and reports nothing. The rows below are therefore the
**complete set of implicit rejection / boundary behaviours** the C code exhibits
— i.e. every condition under which it declines to do work, plus the generic
C-API boundaries mandated by the task (null pointers, zero/oversized lengths,
one-past-range values). Each is a differential test asserting C and Rust behave
*identically*, not merely "both failed".

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `tfm` | `count == 0` (zero length) — loop guard `i < count` false on entry | returns immediately; **zero** bytes read from `src`, **zero** bytes written to `dest`. Rust must leave the destination buffer byte-identical to its pre-call contents. |
| 2 | `tfm` | `count == -1` (negative length, one step past the valid `count >= 0` range) | loop guard false; no reads, no writes, no crash. Identical to row 1. |
| 3 | `tfm` | `count == INT_MIN` (`-2147483648`, extreme negative) | loop guard false; no reads/writes. No wraparound into a huge positive loop count. |
| 4 | `tfm` | `count < 0` for many random negative values | no reads/writes for every one of them. |
| 5 | `tfm` | `dest == NULL`, `src == NULL`, **and** `count <= 0` | pointers are never dereferenced (the loop body never runs), so the null pointers are inert and the call returns normally. Rust must also not fault — in particular it must not form/deref a pointer or panic. |
| 6 | `tfm` | `dest == NULL`, `src` valid, `count == 0` | same as row 5: returns normally, no fault. |
| 7 | `tfm` | `src == NULL`, `dest` valid, `count == 0`; `dest` buffer pre-filled | returns normally, `dest` untouched (still the pre-fill pattern). |
| 8 | `tfm` | `count == 1` with `src` sized to exactly 3 floats and `dest` to exactly 2 floats (minimum non-empty length; no slack) | reads exactly `src[0..3]`, writes exactly `dest[0..2]`. Guard bytes placed immediately after both buffers must be unmodified — i.e. no off-by-one over-read/over-write past `3*count` / `2*count`. |
| 9 | `tfm` | `count == 1`, `sqd < 0` forced (negative discriminant) so the clamp `0 > sqd` is TRUE | `sqrtf` is called on the `0.0f` constant, **not** on the negative value; `errno` is never set and no `NaN` appears. `lambda == 0.5f*(dy2+dx2)`. |
| 10 | `tfm` | `count == 1`, `sqd` is exactly zero (boundary of the clamp: `0 > 0.0f` and `0 > -0.0f` are both FALSE, so `sqd` itself — not the literal `0` — is what reaches `sqrtf`) | `sqd` is *not* clamped; `sqrtf(±0.0f) == ±0.0f`. The sign of the zero must match bit-for-bit. **Reachability, established experimentally:** a `-0.0f` discriminant is *unreachable* from finite inputs, because the final addend `4.0f*dxy*dxy` is `+0.0f` for every finite `dxy` and `x + (+0.0)` is never `-0.0`. The reachable boundary is `sqd == +0.0f` exactly, produced whenever `dx2 == dy2` and `dxy == ±0.0` (the `2*` doubling is exact, so the three terms cancel exactly). Both are covered. |
| 11 | `tfm` | `count == 1`, `sqd` is `NaN` (clamp comparison `0 > NaN` is FALSE, so the NaN reaches `sqrtf`) | NaN propagates through `sqrtf` into `lambda` and into the output. Output bits (including NaN sign and payload) must match. |
| 12 | `tfm` | `src[0]` and/or `src[1]` is `NaN`, so the branch test `src[0] < src[1]` is FALSE (unordered) | the **`else`** branch is taken, never the `if` branch. |
| 13 | `tfm` | inputs at/over the float range: `±FLT_MAX`, `±inf`, so intermediates overflow to `±inf` and `inf - inf` / `0 * inf` produce the invalid-operation NaN | output bits must match exactly, including the NaN's sign bit and payload. |
| 14 | `tfm` | subnormal / `±0.0` inputs (values one step past the normal range: `±FLT_MIN`, `±FLT_TRUE_MIN`, `±0.0`) | underflow-to-zero and signed-zero behaviour must match bit-for-bit. |
| 15 | `tfm` | `dest == src` (fully aliasing pointers) — the C has no `restrict`, and the stride mismatch (`dest += 2` vs `src += 3`) means writes overwrite not-yet-read source elements | the C's exact read/write interleaving must be reproduced: each iteration reads all three inputs before writing two outputs. Buffer contents after the call must match byte-for-byte. |
| 16 | `tfm` | `dest` overlapping `src` at a positive offset / negative offset (partial aliasing, several shifts) | same as row 15: byte-identical buffers afterwards. |
| 17 | `tfm` | *all* inputs `NaN` with distinct non-canonical payloads and mixed sign bits | the surviving NaN payload/sign selected by the C must be reproduced exactly (this is the documented operand-order hazard in `src/lib.rs`). |

Rows 1–8 are the "declines to do work / no out-of-bounds access" rejections;
9–17 are the value-domain boundaries that the clamp, the comparison and the
IEEE-754 invalid operations create.

---

## Verification result — every row has a passing differential test

All in `tests/phase_c_errors.rs`. Each test constructs the exact condition,
calls BOTH `.so`s, and asserts the *same* rejection (identical bytes / identical
refusal to write), not merely "both failed".

| # | test | [x] |
|---|------|-----|
| 1 | `err01_count_zero_writes_nothing` | [x] |
| 2 | `err02_count_negative_one` | [x] |
| 3 | `err03_count_int_min` | [x] |
| 4 | `err04_many_random_negative_counts` | [x] |
| 5 | `err05_both_null_nonpositive_count` | [x] |
| 6 | `err06_null_dest_valid_src_count_zero` | [x] |
| 7 | `err07_null_src_valid_dest_count_zero` | [x] |
| 8 | `err08_minimum_nonempty_exact_buffers` | [x] |
| 9 | `err09_negative_discriminant_is_clamped_not_sqrt_of_negative` | [x] |
| 10 | `err10_minus_zero_discriminant_boundary` | [x] |
| 11 | `err11_nan_discriminant_propagates_through_sqrt` | [x] |
| 12 | `err12_unordered_compare_takes_else_branch` | [x] |
| 13 | `err13_range_extremes_and_invalid_ops` | [x] |
| 14 | `err14_subnormal_and_signed_zero_boundaries` | [x] |
| 15 | `err15_exact_aliasing_dest_equals_src` | [x] |
| 16 | `err16_partial_overlap_offsets` | [x] |
| 17 | `err17_all_nan_distinct_payloads` | [x] |

Plus the generic C-API boundaries required regardless of the table:

| test | covers | [x] |
|------|--------|-----|
| `generic_count_full_int_domain` | `count` over the whole `int` domain: `INT_MIN`, every negative power of two, `-1`, `0`, `1..40`, and sign-bit boundaries. `count` is the API's only scalar parameter and it is a plain `int`; **there is no `enum` anywhere in the API**, so "out-of-range enum variant across FFI" reduces to exactly this test — every `int` bit pattern is fed through and C/Rust agree. | [x] |
| `generic_all_null_and_zero_length_combinations` | all four NULL/non-NULL combinations of `dest`/`src` × zero and negative lengths | [x] |
| `generic_bogus_nonnull_pointers_with_nonpositive_count` | non-NULL but unmapped/misaligned pointers with `count <= 0` (must not be dereferenced by either) | [x] |

### Note on unreachable error paths

`src/lib.rs`'s `fsqrt` contains a negative-argument branch. It is **provably
dead**: `clamp_nonneg_c` returns `0.0f32` whenever `0.0 > sqd`, so `fsqrt` only
ever receives a value `>= 0.0`, `-0.0`, or NaN. This mirrors the C, where the
`comiss`/`jbe` pair replaces any negative `sqd` with the `0.0f` constant before
`sqrtf@plt` is called, so C's `sqrtf` never sets `errno` either. Mutating that
dead branch is a semantic no-op and cannot be caught by any differential test —
confirmed experimentally, and recorded as an intentional exclusion in
`verify.sh`. The *reachable* equivalents are rows 9, 10 and 11.
