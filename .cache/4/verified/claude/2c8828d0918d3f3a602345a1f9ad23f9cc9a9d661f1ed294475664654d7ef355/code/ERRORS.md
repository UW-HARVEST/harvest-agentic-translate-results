# ERRORS.md — Phase A error-surface table

Mechanically derived from the C source. The grep for rejection constructs finds
**no** `RETURN_ERROR`-style macro, **no** `assert`, **no** `NULL` check, **no**
`errno` use, **no** error enum, and **no** explicit range check anywhere:

```
$ grep -nE "assert|NULL|errno|ERROR|exit|abort|if *\(" c_src/src/*.c c_src/include/*.h
c_src/src/match.c:37:    if(total(test, bins) < threshold * total(reference, bins)) return 0;

$ grep -n return c_src/src/*.c
c_src/src/match.c:8:     return sum;                    # total(), not a rejection
c_src/src/match.c:37:    ... return 0;                  # <-- the ONLY explicit rejection
c_src/src/match.c:40:    return spectral_contrast(t, r, bins) >= threshold;
c_src/src/spectral_contrast.c:7:  return sum;           # dot_product(), not a rejection
c_src/src/spectral_contrast.c:19: return dot_product(a, b, length);
```

The library validates nothing. Its whole "error surface" is therefore:

1. the single explicit early `return 0` energy gate (row 1),
2. the boolean threshold verdict that yields `0` (rows 2–5),
3. the *implicit* failure modes the arithmetic can reach — division by a zero /
   NaN / infinite magnitude (rows 6–10),
4. the size/pointer domain boundaries the C code does **not** guard, where it
   either is well defined by omission (loops simply do not execute, rows
   11–15) or leaves the well-defined domain entirely (rows 16–19).

`bins`/`length` are `int`, so any `int` is a legal FFI argument value; there is
no enum parameter anywhere in the API, so the "out-of-range enum value" class
does not exist here. Both size parameters are covered for
negative / zero / one / oversized below.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `match` | `total(test,bins) < threshold*total(reference,bins)` — the energy gate at `match.c:37`. Reached e.g. with `test` all zeros, `reference` all ones, `threshold=0.5`, `bins>=1`. | returns `0` immediately; `spectral_contrast` is **never** called, so neither buffer is preprocessed | `err_row01_energy_gate_rejects` |
| 2 | `match` | Gate passes but `spectral_contrast(t,r,bins) < threshold` (`match.c:40` comparison false) | returns `0` | `err_row02_contrast_below_threshold` |
| 3 | `match` | `threshold = NaN`. Gate: `x < NaN` is false ⇒ **not** rejected, so it proceeds; final `contrast >= NaN` is false. (`comisd`+`jbe` / `setae`: unordered ⇒ CF=1 ⇒ 0.) | returns `0`, and the gate does *not* short-circuit | `err_row03_threshold_nan` |
| 4 | `match` | `threshold = +inf`, data finite. `total*inf = +inf`, and `finite < +inf` is true ⇒ gate rejects. | returns `0` via row-1 path | `err_row04_threshold_pos_inf` |
| 5 | `match` | `threshold = -inf` (negative threshold is never validated). `total*-inf = -inf`; `finite < -inf` false ⇒ proceeds; `contrast >= -inf` true unless contrast is NaN. | returns `1` (or `0` iff contrast is NaN) — i.e. no rejection at all | `err_row05_threshold_neg_inf` |
| 6 | `match` | `total(reference,bins) == 0` **and** `threshold == 0` ⇒ `0*0=0`, `0<0` false ⇒ proceeds ⇒ zero data ⇒ zero magnitude in `normalize` (row 8) | returns `0 >= 0` on a NaN contrast ⇒ `0`; NaN must be produced identically | `err_row06_zero_data_zero_threshold` |
| 7 | `match` | `threshold*total(reference)` is NaN via `0 * ±inf` (e.g. `reference` contains `+inf` and `-inf` so `total`=NaN, or `total`=0 with `threshold=inf`) | `x < NaN` false ⇒ gate does **not** reject; proceeds | `err_row07_gate_nan_product` |
| 8 | `spectral_contrast` | `dot_product(v,v,length) == 0` (all-zero vector ⇒ magnitude `0.0`) ⇒ `normalize` computes `v[i] /= 0.0` | `0.0/0.0` ⇒ NaN (x86 `divsd` default QNaN `0xFFF8000000000000`, narrowed by `cvtsd2ss` to `0xFFC00000`); every element becomes NaN and the return value is NaN. Non-zero elements would give `±inf`. **No error code, no trap.** | `err_row08_zero_magnitude_nan` |
| 9 | `spectral_contrast` | vector contains NaN ⇒ `dot_product` NaN ⇒ `sqrt(NaN)` NaN ⇒ every `v[i] /= NaN` NaN | returns NaN, buffer fully NaN; `sqrt` does **not** set `errno`/raise for QNaN | `err_row09_nan_input` |
| 10 | `spectral_contrast` | magnitude overflows to `+inf` (e.g. elements `3.0e38f`, `length>=2`, so the `float` products overflow) ⇒ `sqrt(+inf)=+inf` ⇒ `v[i]/inf = ±0.0` | returns `+0.0` (or `-0.0`/NaN depending on signs); silently destroys the data | `err_row10_magnitude_overflow` |
| 11 | `spectral_contrast` | `length == 0` | Well defined by omission: all three loops are `for(i=0;i<0;...)`, nothing is dereferenced. Returns `+0.0`, buffers untouched. | `err_row11_sc_length_zero` |
| 12 | `spectral_contrast` | `length < 0` (`-1`, `-2`, `i32::MIN`) | Same as row 11: `i < length` is false at once. Returns `+0.0`, buffers untouched, **no** dereference. | `err_row12_sc_length_negative` |
| 13 | `spectral_contrast` | `a == NULL`, `b == NULL`, `length <= 0` | Returns `+0.0` without dereferencing — the null pointers are never loaded from. | `err_row13_sc_null_ptrs_len_le_zero` |
| 14 | `spectral_contrast` | `a == b` (aliased buffers), `length >= 1` | No aliasing guard: `a` is normalized, then normalized **again** (now a unit vector, so magnitude 1), then `dot_product(a,a)` ⇒ ≈`1.0`. Must match bit-for-bit. | `err_row14_sc_aliased_buffers` |
| 15 | `match` | `test == reference` (aliased), `bins >= 1` | No guard; gate compares `s < threshold*s`; both VLAs get the same content ⇒ contrast ≈ `1.0`. | `err_row15_match_aliased_buffers` |
| 16 | `match` | `bins == 0` | **Outside the defined domain.** `float_t t[0]` is a zero-size VLA (C99 6.7.6.2p5 requires size > 0) and `differentiate(v,0)` executes `v[-1] = 0`, an out-of-bounds store into the frame. Observed: **SIGSEGV** (`exit=139`). Not a reproducible value, so it is asserted as "C crashes / Rust is memory-safe", never as equality. | `err_row16_match_bins_zero_is_ub` (documents + verifies Rust safety) |
| 17 | `match` | `bins < 0` | **Outside the defined domain.** Negative VLA size: the emitted size math is `((long)bins*8 + 15)/16*16` on *unsigned* values, so `rsp` moves *up*; then `memcpy(v, source, (size_t)((long)bins*8))` is a ~2^64-byte copy. Observed from a standalone C driver: SIGSEGV for `bins ∈ {-1,-2,-3,-6,-8,-16}` but a garbage `1` for `bins ∈ {-4,-5}`; observed from the Rust test harness: SIGSEGV for *all* of `{-1,-2,-3,-4,-5,-8,-16}`. **The same input gives a different answer depending on the caller's stack layout** — the definitive proof that there is no C result to match. | `err_row17_match_bins_negative_is_ub` (documents + verifies Rust safety) |
| 18 | `match` | `bins` so large that `float_t t[bins], r[bins]` exhausts the caller's stack (`bins = 300_000` ⇒ 4.8 MB of VLA vs the default 2 MiB thread stack) | Stack exhaustion ⇒ SIGSEGV. This is a limit of the **caller's stack**, not of the library's logic: on a 64 MiB stack the same call completes and must then agree with Rust bit-for-bit. Both halves are asserted. | `err_row18_match_huge_bins` |
| 19 | `match` | `test == NULL` and/or `reference == NULL` with `bins >= 1` | `total()` dereferences the null pointer ⇒ **SIGSEGV** in C. The Rust `.so` dies too, and in the deliverable `release` profile with the *same* signal (11). In a `dev` build Rust's `ub_checks` null-dereference assertion fires first, so the panic-through-`extern "C"` aborts with SIGABRT (6) instead — a debug-only diagnostic, disabled by `[profile.release] debug-assertions = false`. | `err_row19_match_null_pointers` |

## Note on rows 16–19 (undefined behaviour)

Rows 16–19 are the *only* places the C leaves its defined domain, and it does so
by omission rather than by rejecting anything. All four are still executed — in a
**subprocess**, so a fatal signal in the C library cannot take the test runner
down — but what can be asserted differs:

* **Rows 18 and 19 are matched exactly.** Both are ordinary memory faults with a
  comparable outcome: for row 19 both libraries die on the null dereference with
  the same signal (in the profile that has UB checks off), and for row 18 the
  crash is a caller-stack limit, so the row also asserts that with a large enough
  stack the two libraries agree bit-for-bit.
* **Rows 16 and 17 cannot be matched, because the C has no single behaviour to
  match.** `bins <= 0` gives a zero/negative-size VLA plus an out-of-bounds
  `v[-1]` store, and the observed result depends on the *caller's* stack layout:
  `bins = -4`/`-5` returned `1` from a standalone C driver but SIGSEGV'd from the
  Rust test harness. These rows therefore assert the only meaningful invariant —
  **the Rust `.so` must stay memory-safe and deterministic** — and record the
  observed C behaviour. Reproducing a stack smash on purpose would be strictly
  worse than absorbing it.

Every other row (1–15) is fully defined C and is asserted bit-for-bit.

The well-defined domain of the API is therefore:
* `spectral_contrast(a, b, length)` — any `length`; `a`/`b` must be readable for
  `max(length,0)` `float`s.
* `match(test, reference, bins, threshold)` — `bins >= 1`, `test`/`reference`
  readable for `bins` `double`s, and `bins` small enough for two VLAs to fit on
  the stack.
