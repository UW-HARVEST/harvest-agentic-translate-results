# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every rejection mechanism was grepped for in `c_src/src/lib.c` +
`c_src/include/lib.h` (the complete library):

| construct grepped | hits |
|-------------------|------|
| `return` (any value / early return) | **0** |
| `assert` / `static_assert` | **0** |
| `NULL` / null check | **0** |
| `ERROR` / `error` / `errno` / error enum / error macro | **0** |
| `exit` / `abort` | **0** |
| `#define` / `#ifdef` / `#if` | **0** |
| `enum` / `struct` / `typedef` | **0** |
| `if` | **1** — `if (sum > 0.0f)` (line 23) |
| ternary | **1** — `v = (((v) > (0)) ? (v) : (0));` (line 18) |
| min/max constants | **0** |

The single public entry point is

```c
void gaussian_kernel(float *dest, int size, float radius);
```

It returns `void`, validates nothing, and has **no error/rejection return path
of any kind**. Consequently there is no error code or sentinel to compare;
the *only* observable result of an invalid input is *what it does to memory
(or does not do)*, plus "does not trap".

So the error surface of this library is entirely made of **implicit**
rejections: input classes for which one of the two branches degenerates and
the function silently writes nothing, writes fewer/more elements than the
caller asked for, or skips normalisation. Each distinct such condition gets
one row below, exactly as the C computes it, together with the generic FFI
boundary cases (null pointer, zero length, negative/oversized length, one step
past the valid range, and out-of-domain integer values crossing the FFI
boundary — `size` is the only integer parameter and, being a plain `int`,
accepts every one of the 2^32 bit patterns, which is the analogue of the
"out-of-range enum value" case).

Constants used in the expectations (verified against glibc `expf`):

* `arg = sigma*sigma*tetha = 5.76000023f` (`0x40b851ec`)
* `s2  = 1/expf(arg)       = 0.00315111107f` (`0x3b4e82df`)
* `V0  = 1.0f - s2         = 0.996848881f` (`0x3f7f317d`)  ← the value at `r == 0`
* clamp threshold: `v <= 0 ⟺ x*x >= arg ⟺ |x| >= 2.4000001`

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test (`tests/phase_c_errors.rs`) | done |
|---|----------|---------------------------------------------|-------------------|----------------------------------|------|
| E1 | `gaussian_kernel` | `size == 0` → `hsize = 0`, loop runs once (`r = 0`), normalisation loop `for (r=0; r<0; ...)` runs **zero** times | writes exactly **1** float `dest[0] = V0 = 0x3f7f317d` (**unnormalised**); `dest[1..]` untouched. No trap. | `e01_size_zero_single_unnormalised_store` | [x] |
| E2 | `gaussian_kernel` | `size == -1` → `hsize = -1/2 = 0` (C truncation toward zero), loop runs once, normalisation loop runs zero times | identical to E1: `dest[0] = 0x3f7f317d`, unnormalised, nothing else written. No trap. | `e02_size_minus_one_single_unnormalised_store` | [x] |
| E3 | `gaussian_kernel` | `size == -2` → `hsize = -1`, so `-hsize = 1 > hsize` and the kernel loop body **never executes**; `sum` stays `0.0f` so `if (sum > 0.0f)` is false | **no store at all**; the entire `dest` buffer is left byte-identical. No trap. | `e03_size_minus_two_no_stores` | [x] |
| E4 | `gaussian_kernel` | any `size <= -2` (e.g. `-3, -4, -100, -12345`) → `hsize <= -1`, loop never runs, `sum == 0` | **no store at all**, buffer untouched. No trap. | `e04_all_negative_sizes_no_stores` | [x] |
| E5 | `gaussian_kernel` | `size == INT_MIN` (`-2147483648`, the extreme out-of-domain integer across the FFI boundary) → `hsize = -1073741824`, `-hsize = 1073741824 > hsize`, loop never runs. Note the negation does **not** overflow. | **no store at all**, buffer untouched. No trap, no integer-overflow trap. | `e05_int_min_size_no_stores_no_overflow` | [x] |
| E6 | `gaussian_kernel` | `dest == NULL` combined with any `size <= -2` (the only case in which the C never dereferences `dest`) | returns without dereferencing; no trap. | `e06_null_dest_with_negative_size` | [x] |
| E7 | `gaussian_kernel` | `size` **even and positive** (e.g. `2, 4, 8, 64`) → loop runs `2*(size/2)+1 = size+1` times: **one store past** the `size` elements the caller's `size` implies (heap overflow by 4 bytes) | writes `size+1` floats; `dest[size]` receives the **raw, unnormalised** `v` for `r = +hsize` because the normalisation loop only covers `r < size`. This out-of-bounds store is part of the C's observable behaviour and must be reproduced. | `e07_even_size_writes_one_element_past` | [x] |
| E8 | `gaussian_kernel` | `radius == +0.0f` → `rs = sigma/0 = +inf`; at `r == 0`, `x = 0 * inf = NaN` → `v = NaN - s2 = NaN` → `NaN > 0` is **false** → `v = +0.0f`; for `r != 0`, `x = ±inf`, `x*x = +inf`, `expf(+inf) = +inf`, `1/+inf = +0`, `v = -s2 < 0` → `+0.0f` | every written element is `+0.0f` (`0x00000000`), `sum == 0.0f`, so the `if (sum > 0.0f)` branch is skipped ⇒ **no normalisation**. No trap, no NaN ever stored. | `e08_radius_positive_zero` | [x] |
| E9 | `gaussian_kernel` | `radius == -0.0f` → `rs = -inf`; same reasoning as E8 | all written elements `+0.0f`; no normalisation. | `e09_radius_negative_zero` | [x] |
| E10 | `gaussian_kernel` | `radius == NaN` (any payload, quiet or signalling, either sign) → `rs = NaN`, `x = NaN` for **every** `r` (including `r == 0`), `v = NaN` → clamp to `+0.0f` | all written elements `+0.0f`; `sum == 0.0f`; no normalisation. Bit-identical to E8. | `e10_radius_nan_every_payload` | [x] |
| E11 | `gaussian_kernel` | `radius` subnormal/denormal-small enough that `sigma/radius` **overflows** to `+inf` (`radius < 1.6f/FLT_MAX ≈ 4.7e-39`, e.g. `1e-45` = `0x00000001`) | `rs = +inf` ⇒ degenerates exactly to E8: all `+0.0f`, no normalisation. | `e11_radius_subnormal_division_overflows` | [x] |
| E12 | `gaussian_kernel` | `radius == -inf` → `rs = -0.0f`, `x = r * -0.0 = ∓0.0`, `x*x = +0.0`, `v = V0` for **all** `r` | every written element becomes `V0` then is normalised by `1/sum`; because `sum` accumulates `2*hsize+1` terms but only `size` elements are scaled, an even `size` yields `1/(size+1)` per element and leaves `dest[size]` at raw `V0`. No trap. | `e12_radius_negative_infinity` | [x] |
| E13 | `gaussian_kernel` | `radius == +inf` → `rs = +0.0f`; same as E12 | same as E12. | `e13_radius_positive_infinity` | [x] |
| E14 | `gaussian_kernel` | `radius` so large that every `abs(x) < 2.4000001` — the clamp ternary's **false** branch is dead, i.e. no tap is ever rejected | all `size` in-range elements strictly positive, `sum > 0`, normalisation applied. | `e14_clamp_never_taken` | [x] |
| E15 | `gaussian_kernel` | `radius` so small (but with `rs` still finite) that every `r != 0` gives `abs(x) >= 2.4000001` — the clamp fires for **every** off-centre tap | `dest[hsize] = 1.0f` exactly after normalisation, every other in-range element `+0.0f`. `sum == V0 > 0`, so normalisation *does* run — with a finite `rs` the only way to reach `sum == 0` is for the loop not to run at all. | `e15_clamp_taken_for_every_off_centre_tap` | [x] |
| E16 | `gaussian_kernel` | `size == 1` (minimum "valid" size; one step below the smallest size for which the loop writes more than the centre tap) | `hsize = 0`, writes 1 float, `sum = V0 > 0`, normalisation covers `r = 0` ⇒ `dest[0] = 1.0f` exactly (`0x3f800000`). | `e16_size_one_normalises_to_exactly_one` | [x] |
| E17 | `gaussian_kernel` | `size == 2` (one step past `size == 1`; smallest *even* size) | `hsize = 1`, loop writes indices `0,1,2` (3 stores for a 2-element request); `dest[0] = dest[1]` normalised, `dest[2]` raw. Verifies the off-by-one at the boundary. | `e17_size_two_off_by_one_boundary` | [x] |
| E18 | `gaussian_kernel` | fully random 32-bit pattern in `radius` (covers signalling NaNs, negative subnormals, ±inf, huge/tiny normals — i.e. every value a C caller can legally pass through the FFI) crossed with degenerate `size` values | must match C bit-for-bit; in particular no NaN/inf may ever be stored, and `sum > 0.0f` must be evaluated with the same `comiss/ja` (NaN ⇒ false) semantics. | `e18_fuzz_radius_bit_patterns_times_degenerate_sizes` | [x] |

## Notes on things that are *not* rows

* There is no error code, no sentinel return, no `errno` use, and no output
  parameter reporting failure — so "same error code" degenerates to "same
  memory effect and same absence of a trap", which is what the tests assert
  (byte-for-byte over the buffer **plus** guard padding, so a divergent number
  of stores is also caught).
* `dest == NULL` with `size >= -1` is *not* a row: the C unconditionally
  dereferences `dest` there, so it is genuine undefined behaviour / a segfault
  in both implementations and not a comparable "rejection".
* `sum` cannot overflow to `+inf`: each term is `<= V0 < 1`, and `size` is
  bounded by `INT_MAX`, so `sum <= ~2.1e9`.
* `1/sum` cannot overflow: whenever the loop runs with a finite `rs`, the
  `r == 0` tap contributes `V0 ≈ 0.9968`, so `sum >= V0` and `isum <= 1.0032`.

## Generic C-API boundary rows (required by Phase C beyond the table)

| # | boundary | test | done |
|---|----------|------|------|
| G1 | null pointer (the only defined case: `size <= -2`) | `e06_null_dest_with_negative_size` | [x] |
| G2 | zero length (`size == 0`) x every radius class | `e21_zero_length_with_every_radius_class` | [x] |
| G3 | oversized length (`65535 … 1048576`, i.e. far past any sane kernel width) | `e19_oversized_lengths` | [x] |
| G4 | one step past every `size` boundary (`-4 … 5`, each with every radius class, 3 fills, 4 pointer offsets) | `e20_one_step_past_every_documented_size_boundary` | [x] |
| G5 | out-of-domain integer across the FFI boundary (`INT_MIN`, `INT_MIN+1..3`) — the "enum value with no valid variant" analogue for the only integer parameter | `e05_int_min_size_no_stores_no_overflow`, `e18_…` | [x] |
| G6 | out-of-domain float across the FFI boundary: all 2^32 `radius` bit patterns sampled (sNaN, qNaN both signs, ±inf, ±0, subnormals, `FLT_MAX`) | `e10_radius_nan_every_payload`, `e11_radius_subnormal_division_overflows`, `e18_…` | [x] |

## Result

All 18 table rows **and** all 6 generic boundary rows pass. Reproduce with:

```sh
cd translation
cargo build --offline
cargo test  --offline --test phase_c_errors
```

```
running 22 tests
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
e18_fuzz_radius_bit_patterns: 30000 randomized differential draws
[phase_c_errors] differential comparisons so far: 36817
```

Because the API returns `void`, "the same error code" is asserted as *the same
whole-buffer byte image* (including the bytes before `dest` and 16 `f32` of
trailing guard padding) plus the absence of a trap — which is strictly stronger
than comparing a return value, since it also detects a divergent *number* of
stores.
