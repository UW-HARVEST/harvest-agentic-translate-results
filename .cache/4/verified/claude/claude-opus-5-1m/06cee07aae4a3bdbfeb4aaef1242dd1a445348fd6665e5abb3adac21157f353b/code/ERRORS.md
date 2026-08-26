# ERRORS.md — error-surface table (Phase A / gate for Phase C)

Mechanically derived from **every** control-flow and boundary construct in
`c_src/src/lib.c`. The grep below is the complete set of branches in the
library; there are no other `if`s, no `switch`, no loops, and no early exits.

```
$ grep -n 'return\|if\|assert\|NULL\|ERROR' c_src/src/lib.c
4:    if (sample >= 32766.5)
5:        return (int16_t)32767;
6:    if (sample <= -32767.5)
7:        return (int16_t)-32768;
10:    return s;
23:    pcm[0] = mp3d_scale_pcm(a);
33:    pcm[16 * nch] = mp3d_scale_pcm(a);
```

## Shape of this API's error surface

This library has **no error channel at all**:

* `synth_pair` returns `void` — there is no status code, no errno, no sentinel.
* there are **no** `assert`s, **no** null checks, **no** length/size parameters
  to validate, **no** error enums, and **no** `RETURN_ERROR`-style macros.
* there are **no enum parameters**, so the "out-of-range enum value across FFI"
  class does not exist here; the only non-pointer scalar is `int nch`, and every
  `int` value of `nch` is covered by rows 16–18 and 22 below (plus
  `extra_nch_exhaustive_small_and_random_bit_patterns`, which walks
  `nch = -256..=256` and random 32-bit patterns).

Consequently the *rejection* surface is exactly:

1. the two saturation branches in `mp3d_scale_pcm` (the library's only way of
   "refusing" an input value — it clamps instead of erroring),
2. the implicitly-defined fall-through for values the two range checks do not
   catch (NaN), including the C-undefined `float -> int16_t` conversion,
3. the `s -= (s < 0)` correction branch,
4. undefined behaviour reachable from the unvalidated parameters (null / wild
   `pcm`, null / short `z`, `int` overflow of `16 * nch`).

Every row must produce **identical** observable behaviour in C and Rust — the
same clamped sample value, or the same fatal signal for the UB rows.

## Table

`a0` = accumulator stored to `pcm[0]`; `a1` = accumulator stored to
`pcm[16*nch]`. "result" is the `int16_t` written.

| #  | function | trigger (exact invalid input / condition) | expected C result | test |
|----|----------|-------------------------------------------|-------------------|------|
| 1  | `mp3d_scale_pcm` via `pcm[0]` | `a0 >= 32766.5` (line 4 true) | writes `32767` | `err01_clamp_high_out0` |
| 2  | `mp3d_scale_pcm` via `pcm[0]` | `a0 == 32766.5` exactly — boundary is **inclusive** (`>=`) | writes `32767` | `err02_clamp_high_boundary_exact` |
| 3  | `mp3d_scale_pcm` via `pcm[0]` | `a0` = largest float **below** `32766.5` (one ULP past the check) | falls through, writes `32766` | `err03_clamp_high_one_ulp_below` |
| 4  | `mp3d_scale_pcm` via `pcm[0]` | `a0 <= -32767.5` (line 6 true) | writes `-32768` | `err04_clamp_low_out0` |
| 5  | `mp3d_scale_pcm` via `pcm[0]` | `a0 == -32767.5` exactly — boundary is **inclusive** (`<=`) | writes `-32768` | `err05_clamp_low_boundary_exact` |
| 6  | `mp3d_scale_pcm` via `pcm[0]` | `a0` = smallest float **above** `-32767.5` (one ULP past the check) | falls through, writes `-32767` | `err06_clamp_low_one_ulp_above` |
| 7  | `mp3d_scale_pcm` via `pcm[16*nch]` | `a1 >= 32766.5` | writes `32767` at `16*nch` | `err07_clamp_high_out1` |
| 8  | `mp3d_scale_pcm` via `pcm[16*nch]` | `a1 <= -32767.5` | writes `-32768` at `16*nch` | `err08_clamp_low_out1` |
| 9  | `mp3d_scale_pcm` | `a = +INFINITY` (line 4 true for `+inf`) | writes `32767` | `err09_plus_infinity` |
| 10 | `mp3d_scale_pcm` | `a = -INFINITY` (line 6 true for `-inf`) | writes `-32768` | `err10_minus_infinity` |
| 11 | `mp3d_scale_pcm` | `a = NaN` — **both** range checks are false (unordered `comiss` ⇒ `jb` taken), so the C-undefined `(int16_t)(NaN + .5f)` conversion executes; x86-64 `cvttss2si` yields `0x80000000`, truncated to 16 bits ⇒ `0`, and `s < 0` is false | writes `0` | `err11_nan_accumulator` |
| 12 | `synth_pair` | any single one of the 23 read taps of `z` is `NaN` (NaN reaches the accumulator through the arithmetic) | that output becomes `0`; the other output is unaffected unless it shares the tap | `err12_nan_in_each_tap` |
| 13 | `synth_pair` | `z` taps contain both `+inf` and `-inf` so an intermediate `a` becomes `inf - inf = NaN` | affected output `0` | `err13_inf_minus_inf_is_nan` |
| 14 | `mp3d_scale_pcm` | `-0.5 <= a < 0` ⇒ `(int16_t)(a + .5f)` is `0`, so `s < 0` is **false** and no decrement happens (branch boundary of line 9) | writes `0`, not `-1` | `err14_negative_zero_region` |
| 15 | `mp3d_scale_pcm` | `a < 0` and truncation yields `s < 0` ⇒ line 9 subtracts 1 (round-half-down for negatives) | writes `trunc(a+0.5) - 1` | `err15_negative_decrement_branch` |
| 16 | `synth_pair` | `nch == 0` ⇒ `pcm[16*0]` aliases `pcm[0]`; the second store **overwrites** the first | only `a1`'s sample is observable at `pcm[0]` | `err16_nch_zero_aliases_store` |
| 17 | `synth_pair` | `nch < 0` ⇒ `pcm[16*nch]` is a **negative** index, writing *before* `pcm` | writes at the negative offset | `err17_negative_nch` |
| 18 | `synth_pair` | `nch` huge (`nch >= 0x0800_0000`) ⇒ `16 * nch` overflows `int` (C UB; gcc wraps two's-complement) so the store lands ±GiB away from `pcm` | wild store; identical (wrapped) address in both, so identical crash/behaviour | `err18_nch_int_overflow_parity` (subprocess) |
| 19 | `synth_pair` | `pcm == NULL` (no null check exists) ⇒ null store | fatal `SIGSEGV` | `err19_null_pcm_crash_parity` (subprocess) |
| 20 | `synth_pair` | `z == NULL` (no null check exists) ⇒ null load | fatal `SIGSEGV` | `err20_null_z_crash_parity` (subprocess) |
| 21 | `synth_pair` | `z` shorter than 899 `float`s (the code unconditionally reads up to `z[2 + 14*64]`) ⇒ out-of-bounds read | reads past the end identically in both (same indices) | `err21_short_z_reads_same_indices` |
| 22 | `synth_pair` | `nch` = `INT_MIN` / `INT_MAX` / `±2^28` — extreme legal `int` values whose `16*nch` wraps to a *small* offset (`INT_MAX`→`-16`, `INT_MIN`→`0`, `2^28`→`0`), so the wrap is directly observable in-process | store lands at the wrapped offset | `err22_nch_int_overflow_wrap_semantics` |

## Divergence found and fixed

One real divergence was found by row 19 and is worth recording, because it is
invisible in a release build:

* **`pcm == NULL` in a debug build.** The C simply stores to address 0 and dies
  with `SIGSEGV`. rustc's debug assertions inject a language-UB check that turns
  the identical access into a `"null pointer dereference occurred"` panic, which
  aborts with `SIGABRT` — an observably different termination. Since the C
  performs no validation at all, the fix was to stop injecting checks the C does
  not have, via `[profile.dev] debug-assertions = false` in `Cargo.toml`. After
  the fix both libraries terminate with `signal=11` in *both* profiles, which
  `verify_all.sh` re-checks against the release **and** debug cdylib.

## Checklist (Phase C — all rows must have a *passing* differential test)

- [x] 1  `err01_clamp_high_out0`
- [x] 2  `err02_clamp_high_boundary_exact`
- [x] 3  `err03_clamp_high_one_ulp_below`
- [x] 4  `err04_clamp_low_out0`
- [x] 5  `err05_clamp_low_boundary_exact`
- [x] 6  `err06_clamp_low_one_ulp_above`
- [x] 7  `err07_clamp_high_out1`
- [x] 8  `err08_clamp_low_out1`
- [x] 9  `err09_plus_infinity`
- [x] 10 `err10_minus_infinity`
- [x] 11 `err11_nan_accumulator`
- [x] 12 `err12_nan_in_each_tap`
- [x] 13 `err13_inf_minus_inf_is_nan`
- [x] 14 `err14_negative_zero_region`
- [x] 15 `err15_negative_decrement_branch`
- [x] 16 `err16_nch_zero_aliases_store`
- [x] 17 `err17_negative_nch`
- [x] 18 `err18_nch_int_overflow_parity`
- [x] 19 `err19_null_pcm_crash_parity`
- [x] 20 `err20_null_z_crash_parity`
- [x] 21 `err21_short_z_reads_same_indices`
- [x] 22 `err18_nch_int_overflow_parity` (`INT_MIN` / `INT_MAX` cases)
