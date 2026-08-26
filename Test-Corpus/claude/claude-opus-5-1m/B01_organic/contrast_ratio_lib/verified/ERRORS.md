# ERRORS.md — Phase A: error-surface table

## Mechanical derivation

Every rejection/error construct was grepped for in the **entire** C source
(`c_src/src/lib.c`, 29 lines; `c_src/include/lib.h`, 7 lines):

```
$ grep -nE 'RETURN_ERROR|return *-1|return *NULL|return *0;|assert|errno|abort|exit\(' c_src/src/lib.c c_src/include/lib.h
(no matches)

$ grep -cE '\bif\b' c_src/src/lib.c        # -> 1   (the High/Low swap, not an error check)
$ grep -cE 'enum'  c_src/src/lib.c c_src/include/lib.h   # -> 0
$ grep -cE '\*'    c_src/include/lib.h     # -> 0   (no pointer parameters anywhere)
$ grep -cE 'MIN|MAX|LIMIT|_MAX|_MIN' c_src/src/lib.c c_src/include/lib.h  # -> 0
```

**Finding: this library has NO explicit error surface.**

* There is exactly one public entry point, `contrast_ratio`.
* It returns `float`, not a status code — there is no error sentinel and no
  `errno` use.
* It takes **no pointers**: both parameters are 3-byte structs passed *by value*.
  A null pointer is therefore not an expressible input to this API.
* There are **no enums**, so there is no out-of-range-enum input to construct.
* There are **no length/size/count parameters**, so there is no zero-length or
  oversized-length input to construct.
* Every one of the 3 struct fields is `unsigned char`; **all 256 values of each
  field are in range**. There is no "one step past the valid range" value that a
  caller can express — `256` is not representable in the parameter type, and the
  C code performs no range check because none is needed.

Consequently the table below enumerates the only ways this C code can produce a
non-finite / degenerate result, plus the generic FFI-boundary conditions the
prompt requires. These are derived from what the C *actually* does
(unguarded `High / Low` on line 21 and the by-value struct ABI), not from docs.

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| E1 | `contrast_ratio` | **Division by zero, 0/0.** `A == {0,0,0}` and `B == {0,0,0}`. `cbLuminance` returns `+0.0f` for both; `High = LumA = +0.0`, `Low = LumB = +0.0`; `High < Low` is false; line 21 evaluates `+0.0f / +0.0f`. There is no divide-by-zero guard in the C. | `NaN` (x86 `divss` 0/0 -> default QNaN, bit pattern `0xFFC00000`). Must match Rust **bit for bit**, including the sign bit. |
| E2 | `contrast_ratio` | **Division by zero, x/0 with `B` black.** `B == {0,0,0}`, `A != {0,0,0}` (so `LumA > 0`). `High = LumA`, `Low = +0.0`; `High < Low` false; `LumA / +0.0f`. | `+inf` (`0x7F800000`). No error is returned. |
| E3 | `contrast_ratio` | **Division by zero, x/0 with `A` black (swap path).** `A == {0,0,0}`, `B != {0,0,0}`. `High = LumA = +0.0`, `Low = LumB > 0`; `High < Low` is TRUE so the swap on line 19 runs: `High = LumB`, `Low = LumA = +0.0`; `LumB / +0.0f`. | `+inf` (`0x7F800000`). Exercises the *swap* branch of the same division-by-zero, which E2 does not. |
| E4 | `contrast_ratio` | **Near-zero denominator (no underflow guard).** `B` is the darkest non-black colour `{0,0,1}` and `A == {255,255,255}`; the C divides by `0.0722f * (1/255)/12.92 ≈ 2.2e-5` with no clamping / epsilon check. | A large finite ratio (~`3.5e4`); must match bit for bit. Confirms the absence of any epsilon guard is reproduced rather than "fixed". |
| E5 | `contrast_ratio` | **Garbage in the struct padding bits of `A`.** `cb_rgb_255` occupies 3 of the 8 bytes of the argument register (x86-64 SysV INTEGER class). Caller passes `0xDEADBEEFCC << 24 \| rgb` — i.e. all 5 padding bytes set to non-zero. The C reads only `-0x8/-0x7/-0x6`, i.e. the low 3 bytes. | Identical to the call with zeroed padding. Any Rust divergence here is a `#[repr(C)]` ABI-classification bug. |
| E6 | `contrast_ratio` | **Garbage in the struct padding bits of `B`** (same as E5 but on the second argument register, `rsi`). | Identical to the call with zeroed padding. |
| E7 | `contrast_ratio` | **Both arguments' padding bits garbage, simultaneously, with all-`0xFF` padding** (`!0u64` padding). Catches a Rust codegen that masks one argument but not the other. | Identical to the call with zeroed padding. |
| E8 | `contrast_ratio` | **`0.04045` branch boundary — no clamping.** The `R > 0.04045` test on lines 6-8 has no tolerance; `n/255.f` crosses it between `n = 10` (`0.039215688`, linear branch) and `n = 11` (`0.043137256`, `pow` branch). Feeding exactly `10` and `11` in each of the 6 channel positions probes the un-guarded strict `>`. | The linear result for `n <= 10` and the `pow` result for `n >= 11`, bit-exact, in every channel position. A Rust `>=` instead of `>`, or an f32-vs-f64 comparison, diverges here. |

## Notes on rows that do NOT exist for this API

Recorded explicitly so the absence is a *derived* fact, not an oversight:

* **Null pointers** — impossible: no parameter has pointer type
  (`grep -c '\*' c_src/include/lib.h` -> 0). Tested anyway at the ABI level via
  E5-E7, which is the closest expressible analogue (junk in the by-value
  argument registers).
* **Zero / oversized lengths** — impossible: no length, size, count, stride or
  buffer parameter exists.
* **Out-of-range enum values across FFI** — impossible: `grep -c enum` -> 0. The
  only scalar parameter type is `unsigned char`, whose entire 0..=255 domain is
  valid; the exhaustive per-channel sweeps in `CONFIGS.md` (rows C11-C13) cover
  100 % of that domain, so there is no unrepresented value left to pass.
* **Error codes / sentinels** — impossible: the return type is `float` and the C
  never distinguishes success from failure.

## Phase C results

Every row has a dedicated differential test in `tests/phase_c_errors.rs` that
constructs the exact condition, calls **both** `.so`s, and asserts the **same
sentinel** — the exact `f32` bit pattern, not merely "both failed".

| #  | test | result |
|----|------|--------|
| E1 | `e1_black_vs_black_is_bit_identical_nan` | [x] pass — both return `NaN` with the identical bit pattern `0xFFC00000` (x86 `divss` default QNaN), verified over 1000 repeats and through the junk-padding path |
| E2 | `e2_b_black_returns_positive_infinity` | [x] pass — both return `+inf` (`0x7F800000`) for every non-black grey and every non-black single-channel colour (exhaustive in `n`), plus 20 000 random colours |
| E3 | `e3_a_black_swap_branch_returns_positive_infinity` | [x] pass — same, via the `High < Low` swap route |
| E4 | `e4_near_zero_denominator_has_no_guard` | [x] pass — the huge finite ratio is reproduced exactly; no epsilon guard was introduced (a `low.max(1e-7)` clamp is caught by the suite, see the mutation audit) |
| E5 | `e5_junk_padding_in_argument_a` | [x] pass — 6 junk patterns x 5 000 random colours; padding never affects the result on either side |
| E6 | `e6_junk_padding_in_argument_b` | [x] pass — 4 junk patterns x 5 000 random colours |
| E7 | `e7_junk_padding_in_both_arguments` | [x] pass — 4 padding pairs x 20 000 random colours + the 4 degenerate corners with all-ones padding |
| E8 | `e8_linearization_branch_boundary_is_strict` | [x] pass — `{0,1,9,10,11,12,13,254,255}` in all 6 positions x 5 backgrounds, plus the full 2^6 cross product of `{10,11}`. Includes a non-vacuity assertion that `10` and `11` really do produce different C results. |

### Generic FFI-boundary rows (required regardless of the table)

| test | what it covers | result |
|------|----------------|--------|
| `generic_full_domain_and_range_extremes` | 100 % of the `unsigned char` domain (`0..=255`) in every one of the 6 positions, crossed with the `0`/`255` extremes of the others. This is the complete "one past the range" story for this API: no out-of-range value is representable, so the whole range is covered instead. | [x] pass |
| `generic_raw_register_bit_patterns` | 100 000 fully random 64-bit argument registers + 49 pathological ones (all-zeros, all-ones, `0xAAAA…`, `0x5555…`, sign-bit-only, low-24-only, padding-only), each cross-checked against the clean struct-typed call. This is the closest expressible analogue of "null pointer / out-of-range enum" for an API that has neither. | [x] pass |
| `generic_purity_and_interleaving` | 2 000 inputs x 8 rounds, interleaving C and Rust calls, asserting no drift — rules out hidden mutable state or a lazily-initialised table in the translation. | [x] pass |

**All 8 `ERRORS.md` rows and all 3 generic rows pass, under every feature
combination and in both cargo profiles.**
