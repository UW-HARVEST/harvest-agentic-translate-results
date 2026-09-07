# ERRORS.md — Error / rejection surface table

Derived mechanically from the C source. The library declares **no** error enum,
uses **no** `assert`, **no** `RETURN_ERROR` macro, **no** `NULL` check and
returns **no** negative sentinel. `grep -n 'return\|assert\|NULL\|-1' c_src/src
c_src/include` yields exactly the statements listed below, so the complete
rejection surface consists of:

1. the single explicit rejection branch in `match` (`return 0`);
2. the implicit degenerate paths taken when a loop bound makes the loop body
   execute zero times (`length <= 0`);
3. the out-of-bounds / division-by-zero / non-finite arithmetic conditions the C
   walks into without checking.

Every row below is one distinct rejection or degenerate branch, with the exact
result the compiled C produces (verified differentially in
`tests/error_paths.rs`).

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `match` (`match.c:37`) | energy gate fails: `total(test,bins) < threshold * total(reference,bins)` | returns `0`, **no** preprocessing and **no** call to `spectral_contrast`; `test`/`reference` left untouched | `err01_energy_gate_rejects` |
| 2 | `match` (`match.c:37`) | gate comparison is *unordered*: `threshold` is NaN, or `threshold*total(ref)` is NaN (`0 * ±inf`), or `total(test)` is NaN | `comisd` unordered ⇒ `jbe` taken ⇒ gate does **not** reject; falls through to `spectral_contrast` | `err02_gate_unordered_falls_through` |
| 3 | `match` (`match.c:40`) | final comparison unordered: `spectral_contrast(...)` is NaN or `threshold` is NaN | `setae` after unordered `comisd` (CF=1) ⇒ returns `0` | `err03_final_compare_unordered_returns_0` |
| 4 | `match` (`match.c:36-40`) | `bins == 0` — GCC allocates zero-length VLAs, so `rsp` is unchanged and `t` aliases the stack top. `differentiate` then executes `v[length-1] = 0`, i.e. **`v[-1] = 0`**, which lands exactly on `preprocess`'s saved return address | **UB: the C reproducibly SIGSEGVs** (returns to address 0). Verified out-of-process. Documented divergence: the Rust has no VLA to overrun and returns `(0.0 >= threshold)` | `err04_bins_zero` |
| 5 | `match` (`match.c:36`) | `bins < 0` — VLA with negative element count: `size = (bins*8 + 15) / 16 * 16` computed with **unsigned** division, so `rsp` is decremented by ~2^64, corrupting the stack | undefined behaviour; process crashes (SIGSEGV). Not reproducible by definition — documented and probed out-of-process only, never asserted for value equality | `err05_bins_negative_out_of_process` |
| 6 | `total` (`match.c:7`) | `length <= 0` | loop body never runs, returns `+0.0`. Only reachable via `bins <= 0` (rows 4/5 UB); the structurally identical `dot_product` zero-trip accumulator IS reachable and returns exactly `+0.0` | `err06_total_nonpositive_length` |
| 7 | `smoothen` (`match.c:14`) | `length <= 0` | outer loop never runs, buffer untouched (only reachable via `bins <= 0`, which is row 4/5 UB; the structurally identical zero-trip loop in `normalize` is reachable and verified) | `err04_bins_zero`, `err15_spectral_contrast_nonpositive_length` |
| 8 | `smoothen` (`match.c:16-18`) | tail window truncation: `i + N_SMOOTH > length`, so fewer than `N_SMOOTH` samples are summed but the divisor is **still `N_SMOOTH`** (never renormalised) | tail values are systematically attenuated; for `length < 16` *every* output is attenuated | `err08_smoothen_tail_divisor` |
| 9 | `differentiate` (`match.c:24-25`) | `length == 0` ⇒ `for(i = 0; i < -1; i++)` never runs and `v[-1] = 0` writes out of bounds | the store clobbers the caller's saved return address ⇒ SIGSEGV (same event as row 4) | `err04_bins_zero` |
| 10 | `differentiate` (`match.c:24-25`) | `length == 1` ⇒ loop never runs, `v[0] = 0` | sole element forced to `0.0` | `err10_differentiate_length_one` |
| 11 | `differentiate` (`match.c:24`) | `v[i+1]` and `v[i]` both `+inf` (or both `-inf`) ⇒ `subsd` of equal infinities | element becomes NaN (x86 indefinite `-nan`, `0xFFF8…`) and propagates through the second `smoothen` | `err11_inf_minus_inf_nan` |
| 12 | `normalize` (`spectral_contrast.c:11-13`) | `magnitude == 0.0` (every lane is `±0`; via `match` this means the all-zero `double` vector) ⇒ `v[i] /= 0.0` | `0/0 → -nan` (`0xFFF8…`, truncated to `float` `0xFFC00000`), `x/0 → ±inf`; `dot_product` then returns NaN, so `match` returns `0` for every threshold | `err12_zero_magnitude`, `err03_final_compare_unordered_returns_0` |
| 13 | `normalize` (`spectral_contrast.c:11`) | `dot_product(v,v)` is NaN (a NaN or an `inf*0` lane) ⇒ `sqrt(NaN)` | glibc `sqrt` = `sqrtsd`: returns the input NaN quieted, payload preserved; all elements become NaN | `err13_sqrt_of_nan` |
| 14 | `normalize` (`spectral_contrast.c:11`) | `dot_product(v,v)` overflows to `+inf` (large `float` lanes) ⇒ `magnitude = +inf` | every `v[i] /= inf → ±0`; `dot_product` then returns `+0.0` | `err14_magnitude_infinite` |
| 15 | `spectral_contrast` (`spectral_contrast.c:16-19`) | `length <= 0` (`0`, `-1`, `INT_MIN`) | all three loops execute zero times; `sqrt(0.0) = 0.0`; returns `+0.0`; **no** memory is dereferenced, so even `NULL` pointers are accepted | `err15_spectral_contrast_nonpositive_length` |
| 16 | `spectral_contrast` (`spectral_contrast.c:16-19`) | `a == b` (aliased pointers) | `a` is normalised twice: the second `normalize` divides the already-unit vector by its own magnitude (`≈1`), then `dot_product(a,a)` ⇒ `≈1.0` | `err16_spectral_contrast_aliased` |
| 17 | `match` (`match.c:38-40`) | the `float_t` mismatch itself: `spectral_contrast` receives `bins` **as a `float` count** for buffers holding `bins` **doubles**, so it walks them with a 4-byte stride — lane `2k` is the low half of `double` `k`, lane `2k+1` its high half — covering only the first `ceil(bins/2)` doubles and writing normalised `float`s back over those bytes | not a rejection, but the defining wrong-type condition; must be reproduced exactly | `err17_float_t_mismatch_is_reproduced` + every `CONFIGS.md` row |
| 18 | `match` / `spectral_contrast` | `NULL` pointer with a positive length | dereferences `NULL` ⇒ SIGSEGV in both implementations; probed out-of-process only | `err18_null_with_positive_length_out_of_process` |
| 19 | `match` (`match.c:36`) | `bins` so large the VLA exceeds the stack (`bins ≥ ~10^6` on an 8 MiB stack) | stack overflow ⇒ SIGSEGV in C. The Rust uses heap `Vec`, so this is a deliberate, documented divergence in *crash* behaviour only (C UB); values are never compared there | `err19_huge_bins_documented` |
| 20 | `spectral_contrast` "enum"/out-of-range integer | there is no enum in this API; the only integer parameter is `int length` / `int bins`. Values one past every boundary are covered: `INT_MIN`, `-1`, `0`, `1`, `2`, `15`, `16`, `17`, `INT_MAX` (the last two only for `spectral_contrast`, whose non-positive/valid split is the whole domain) | see rows 4, 5, 10, 15 | `err20_integer_boundaries` |

## Checklist

- [x] 1 `err01_energy_gate_rejects`
- [x] 2 `err02_gate_unordered_falls_through`
- [x] 3 `err03_final_compare_unordered_returns_0`
- [x] 4 `err04_bins_zero`
- [x] 5 `err05_bins_negative_out_of_process`
- [x] 6 `err06_total_nonpositive_length`
- [x] 7 `err04_bins_zero`
- [x] 8 `err08_smoothen_tail_divisor`
- [x] 9 `err04_bins_zero`
- [x] 10 `err10_differentiate_length_one`
- [x] 11 `err11_inf_minus_inf_nan`
- [x] 12 `err12_zero_magnitude`
- [x] 13 `err13_sqrt_of_nan`
- [x] 14 `err14_magnitude_infinite`
- [x] 15 `err15_spectral_contrast_nonpositive_length`
- [x] 16 `err16_spectral_contrast_aliased`
- [x] 17 `err17_float_t_mismatch_is_reproduced`
- [x] 18 `err18_null_with_positive_length_out_of_process`
- [x] 19 `err19_huge_bins_documented`
- [x] 20 `err20_integer_boundaries`
