# ERRORS.md — Phase C error/rejection surface table

Derived mechanically from the full text of `c_src/src/lib.c` (34 lines) and
`c_src/include/lib.h` (5 lines). Exhaustive grep for every rejection construct:

```
$ grep -nE 'return|assert|NULL|errno|error|ERROR|if *\(|<=|>=|INT|MAX|MIN|#if' \
      c_src/src/lib.c c_src/include/lib.h
src/lib.c:4:    if (sample >= 32766.5)
src/lib.c:5:        return (int16_t)32767;
src/lib.c:6:    if (sample <= -32767.5)
src/lib.c:7:        return (int16_t)-32768;
src/lib.c:10:    return s;
```

Findings that constrain the table:

- There is **no error-return macro** (`RETURN_ERROR` etc.), **no error enum**,
  **no `assert`**, **no `NULL` check**, **no `errno` use**, and **no `#if`
  conditional** anywhere in the C.
- `synth_pair` returns `void`, so it has no error channel at all. Its only
  observable output is the two `int16_t` stores.
- The complete rejection surface of the library is therefore the **saturating
  clamp** inside `mp3d_scale_pcm` (`src/lib.c:4-7`) — the two branches that
  reject out-of-representable-range float input — plus the *implicit* /
  undefined-behaviour boundaries that any C API of this shape has. Those are
  listed below too, as the task requires, with the concrete behaviour the
  compiled C actually exhibits on this target (x86-64, SSE `cvttss2si`).

Each row is exercised by a differential test in `tests/differential.rs` that
calls **both** `.so`s through `libloading` and asserts the produced `int16_t`
pair is bit-identical.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `mp3d_scale_pcm` via `synth_pair` (`pcm[0]`) | accumulator `a >= 32766.5` (upper clamp branch, `src/lib.c:4`) | returns `32767` (`INT16_MAX`); `pcm[0] == 32767` |
| 2 | `mp3d_scale_pcm` via `synth_pair` (`pcm[16*nch]`) | accumulator `a >= 32766.5` on the second (odd) accumulation | `pcm[16*nch] == 32767` |
| 3 | `mp3d_scale_pcm` via `synth_pair` (`pcm[0]`) | accumulator `a <= -32767.5` (lower clamp branch, `src/lib.c:6`) | returns `-32768` (`INT16_MIN`); `pcm[0] == -32768` |
| 4 | `mp3d_scale_pcm` via `synth_pair` (`pcm[16*nch]`) | accumulator `a <= -32767.5` on the second accumulation | `pcm[16*nch] == -32768` |
| 5 | `mp3d_scale_pcm` | `a` exactly `== 32766.5` — boundary is `>=`, i.e. inclusive (one step *inside* the clamp) | clamps: `32767` |
| 6 | `mp3d_scale_pcm` | `a` exactly `== -32767.5` — boundary is `<=`, inclusive | clamps: `-32768` |
| 7 | `mp3d_scale_pcm` | `a` one representable `f32` step *below* `32766.5` (`32766.498046875`), i.e. one step past the clamp range on the valid side | no clamp: `(int16_t)(a+.5f) = 32766`, non-negative so no decrement → `32766` |
| 8 | `mp3d_scale_pcm` | `a` one representable `f32` step *above* `-32767.5` (`-32767.498046875`) | no clamp: `(int16_t)(a+.5f) = -32766`, negative → `-32767` |
| 9 | `mp3d_scale_pcm` | `a` negative but `> -1.0` (e.g. `-0.25`): `sample+.5f` truncates to `0`, so `s == 0` and `(s < 0)` is **false** | `0` (the decrement does *not* fire — the sign test is on the truncated value) |
| 10 | `mp3d_scale_pcm` | `a == -0.0f` (signed zero) | `-0.0f + .5f = 0.5f` → `(int16_t)0` → `s<0` false → `0` |
| 11 | `mp3d_scale_pcm` | `a` in `[-1.0, -0.5]` e.g. `-1.0`: `-1.0+.5f = -0.5f` → truncates toward zero to `0` → not `< 0` | `0`, **not** `-1` |
| 12 | `mp3d_scale_pcm` | `a == NaN` (produced by `+Inf + -Inf` taps, or a NaN tap). Both comparisons are false, so the `(int16_t)` cast of NaN is reached — formally UB | compiled C uses `cvttss2si` → `0x80000000`, narrowed to `int16_t` → `0`; then `s<0` false → `0` |
| 13 | `mp3d_scale_pcm` | `a == +Inf` (from `+Inf` tap) | `+Inf >= 32766.5` → upper clamp → `32767` |
| 14 | `mp3d_scale_pcm` | `a == -Inf` | `-Inf <= -32767.5` → lower clamp → `-32768` |
| 15 | `synth_pair` | `nch == 0` → the two stores **alias** (`pcm[0]` then `pcm[0]`); the second write wins | `pcm[0]` holds the *second* (odd) accumulator's scaled value |
| 16 | `synth_pair` | `nch < 0` (e.g. `-1`, `-2`) — `16*nch` is a negative `int` index; C pointer arithmetic writes *before* `pcm` | writes at `pcm - 16*|nch|`; must not wrap through `usize` |
| 17 | `synth_pair` | `nch` at the signed-int extremes / large magnitude (`INT_MAX`, `INT_MIN`) — `16 * nch` overflows `int` (UB) | compiled C computes `16*nch` with wrapping 32-bit `imul`, then sign-extends; Rust must match the same target offset |
| 18 | `synth_pair` | `pcm == NULL` | null-pointer store — segfault in both; documented as UB, asserted only via "not called" (not exercised as a crash test) |
| 19 | `synth_pair` | `z == NULL` | null-pointer load — segfault in both; documented as UB, not exercised |
| 20 | `synth_pair` | zero-length / undersized `z` (fewer than `2 + 14*64 + 1 = 899` floats) | out-of-bounds read — UB in both; not exercised as a crash test, but the *minimum in-bounds* size `899` **is** exercised to prove neither side touches a tap beyond `z[898]` |
| 21 | `synth_pair` | all taps `0.0f` (degenerate but valid input; the only input for which both accumulators are exactly `0`) | `pcm[0] == 0`, `pcm[16*nch] == 0` |
| 22 | `mp3d_scale_pcm` | subnormal / tiny taps (`f32::MIN_POSITIVE`, `1e-45`) — accumulator stays subnormal, `+.5f` → `0.5f` → `0` | `0` on both stores |

Rows 18–20 are genuine C undefined behaviour (a null deref / OOB read); a
differential test cannot compare a segfault to a segfault inside one process.
They are recorded for completeness and handled by *proving the boundary is not
crossed*: row 20's minimum-size test allocates exactly `899` floats in a guarded
allocation, so any extra tap read by either implementation would fault.

**Out-of-range enum values:** the C API declares **no `enum` parameters** —
`synth_pair`'s only integer parameter is a plain `int nch`, whose entire
`INT_MIN..=INT_MAX` domain is legal input to the C. Rows 15–17 cover that domain
(zero, negative, and the overflowing extremes) as the equivalent of the
out-of-range-enum check.

## Row → test mapping (all passing)

| row(s) | test in `tests/error_paths.rs` | status |
|---|---|---|
| 1 | `err01_upper_clamp_store0` | [x] |
| 2 | `err02_upper_clamp_store_nch` | [x] |
| 3 | `err03_lower_clamp_store0` | [x] |
| 4 | `err04_lower_clamp_store_nch` | [x] |
| 5 | `err05_upper_boundary_is_inclusive` | [x] |
| 6 | `err06_lower_boundary_is_inclusive` | [x] |
| 7 | `err07_one_step_below_upper_boundary_does_not_clamp` | [x] |
| 8 | `err08_one_step_above_lower_boundary_does_not_clamp` | [x] |
| 9 | `err09_small_negative_does_not_decrement` | [x] |
| 10 | `err10_negative_zero_accumulator` | [x] |
| 11 | `err11_minus_one_truncates_to_zero` | [x] |
| 12 | `err12_nan_accumulator_reaches_the_cast` | [x] |
| 13 | `err13_positive_infinity_takes_upper_clamp` | [x] |
| 14 | `err14_negative_infinity_takes_lower_clamp` | [x] |
| 15 | `err15_nch_zero_aliases_stores_second_wins` | [x] |
| 16 | `err16_negative_nch_writes_before_pcm` | [x] |
| 17 | `err17_nch_extremes_wrap_16x_in_int32` | [x] |
| 18, 19 | `err18_19_null_pointers_are_undefined_and_not_dereferenced` (structural: proves neither side added a check the other lacks) | [x] |
| 20 | `err20_minimum_z_length_no_overread` + `row17_minimum_inbounds_z_buffer` (page-guarded) | [x] |
| 21 | `err21_all_zero_taps_yield_zero` | [x] |
| 22 | `err22_subnormal_taps` | [x] |
| generic FFI boundary | `generic_nch_full_domain_sample` (covers the whole `int` domain of `nch` via the `nch = m*2^28 + s` wrap identity), `generic_zero_and_boundary_lengths` (oversized `z` must be ignored past index 898) | [x] |

## Divergence found and fixed

`src/lib.rs` originally computed the second store's index as
`16isize * nch as isize` -- a **64-bit** multiply. The C evaluates `16 * nch` in
`int` and only then widens (GCC emits `shl $0x4,%eax; cltq`, i.e. wrap modulo
2^32 then sign-extend). For every `|nch| >= 2^27` the two disagree. Fixed to
`nch.wrapping_mul(16) as isize`.

Negative control: reverting that single line makes the suite fail immediately
(`row16`, `row20`, `err17` -- the process aborts with `SIGSEGV` because the
64-bit offset writes far out of bounds), confirming the tests detect it.
