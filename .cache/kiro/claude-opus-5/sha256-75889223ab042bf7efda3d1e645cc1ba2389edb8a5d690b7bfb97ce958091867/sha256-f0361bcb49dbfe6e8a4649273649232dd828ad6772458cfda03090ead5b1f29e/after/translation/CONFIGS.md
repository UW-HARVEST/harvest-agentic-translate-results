# CONFIGS.md — Phase A configuration-surface table

Mechanically derived from `c_src/include/lib.h` and `c_src/src/lib.c`.

## Axes the C code actually branches on

The public API is exactly one entry point (there are no convenience wrappers and
no lower layer to reach past — `include/lib.h` is one line):

```c
void normalize(float *dest, const float *src, int size);
```

There is **no runtime option / mode / flag**: no struct of settings, no global,
no `#ifdef` in `src/lib.c`, no `switch`. `grep -n '#if' src/lib.c` → no matches.
So the configuration axes are purely the *input shape* and *pointer topology*
that the three branches at lines 9, 11 and 15 distinguish:

| axis | values the code distinguishes |
|------|-------------------------------|
| A. `size` | `< 0`, `0`, `1`, `2`, small (3–8), medium (9–64), large (65–4096), odd/even (vectorization boundary), non-multiple-of-vector-width tails |
| B. pointer topology | `dest != src` (disjoint), `dest == src` (full in-place), `dest` overlaps `src` forward (`dest = src + k`), `dest` overlaps `src` backward (`src = dest + k`) |
| C. `sum` classification (line 11 `sum > 0.0f`) | `sum > 0` finite, `sum == +inf` (overflow), `sum == 0` from all-zero input, `sum == 0` from underflow, `sum == NaN` |
| D. element value class | `[-1,1]` uniform, small ints, huge finite (`~1e19`+, square overflows), tiny/subnormal (square underflows), mixed signs, `±0.0`, `±inf`, `NaN`, arbitrary random bit patterns, mixed magnitudes (catastrophic-cancellation-free but rounding-sensitive accumulation order) |
| E. buffer alignment | `dest`/`src` 16-byte aligned vs offset by 1/2/3 floats (release-build auto-vectorization of the store loop is alignment-sensitive) |

Rows below are the pruned cross-product: one row per combination the C treats
differently. Every row is exercised with **many randomized inputs (fixed seed
`0x9E3779B97F4A7C15`)**, both `.so`s loaded via `libloading`, comparing all
`size` output floats as raw `u32` bit patterns (so NaN payloads and signed zero
are compared exactly), plus canary bytes on both sides of `dest`.

## Configuration surface

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `normalize` | A=`0`, B=disjoint, D=n/a | `cfg_row01_size0_disjoint` | [x] |
| 2 | `normalize` | A=`0`, B=in-place | `cfg_row02_size0_inplace` | [x] |
| 3 | `normalize` | A=`1`, B=disjoint, C=`sum>0`, D=uniform `[-1,1]` | `cfg_row03_size1_disjoint_uniform` | [x] |
| 4 | `normalize` | A=`1`, B=in-place, C=`sum>0`, D=uniform | `cfg_row04_size1_inplace_uniform` | [x] |
| 5 | `normalize` | A=`1`, B=disjoint, C=`sum==0`, D=`±0.0` | `cfg_row05_size1_zero` | [x] |
| 6 | `normalize` | A=`2`, B=disjoint, C=`sum>0`, D=uniform | `cfg_row06_size2_disjoint_uniform` | [x] |
| 7 | `normalize` | A=small `3..8`, B=disjoint, C=`sum>0`, D=uniform | `cfg_row07_small_disjoint_uniform` | [x] |
| 8 | `normalize` | A=small `3..8`, B=in-place, C=`sum>0`, D=uniform | `cfg_row08_small_inplace_uniform` | [x] |
| 9 | `normalize` | A=medium `9..64`, B=disjoint, C=`sum>0`, D=uniform | `cfg_row09_medium_disjoint_uniform` | [x] |
| 10 | `normalize` | A=medium `9..64`, B=in-place, C=`sum>0`, D=uniform | `cfg_row10_medium_inplace_uniform` | [x] |
| 11 | `normalize` | A=large `65..4096`, B=disjoint, C=`sum>0`, D=uniform | `cfg_row11_large_disjoint_uniform` | [x] |
| 12 | `normalize` | A=large `65..4096`, B=in-place, C=`sum>0`, D=uniform | `cfg_row12_large_inplace_uniform` | [x] |
| 13 | `normalize` | A=`1..4096`, B=disjoint, D=**arbitrary random bit patterns** (hits `NaN`, `inf`, subnormal, huge, tiny in the same buffer → C in `{sum>0, +inf, 0, NaN}` chosen by data) | `cfg_row13_random_bitpatterns_disjoint` | [x] |
| 14 | `normalize` | A=`1..4096`, B=in-place, D=arbitrary random bit patterns | `cfg_row14_random_bitpatterns_inplace` | [x] |
| 15 | `normalize` | A=`1..512`, B=disjoint, C=`sum==+inf`, D=huge finite (`|x| >= 1e19`) | `cfg_row15_huge_overflow_disjoint` | [x] |
| 16 | `normalize` | A=`1..512`, B=in-place, C=`sum==+inf`, D=huge finite | `cfg_row16_huge_overflow_inplace` | [x] |
| 17 | `normalize` | A=`1..512`, B=disjoint, C=`sum==0` via underflow, D=tiny/subnormal (`|x| <= 1e-25`) | `cfg_row17_tiny_underflow_disjoint` | [x] |
| 18 | `normalize` | A=`1..512`, B=in-place, C=`sum==0` via underflow, D=tiny/subnormal | `cfg_row18_tiny_underflow_inplace` | [x] |
| 19 | `normalize` | A=`1..512`, B=disjoint, C=`sum` finite but *barely* `>0` (mixed subnormal + normal so `sum` is subnormal) | `cfg_row19_subnormal_sum_disjoint` | [x] |
| 20 | `normalize` | A=`1..512`, B=disjoint, D=mixed magnitudes spanning `1e-20 .. 1e20` (accumulation-order / rounding sensitive) | `cfg_row20_mixed_magnitude_disjoint` | [x] |
| 21 | `normalize` | A=`1..512`, B=disjoint, D=contains explicit `±inf` elements alongside finite | `cfg_row21_inf_elements_disjoint` | [x] |
| 22 | `normalize` | A=`1..512`, B=disjoint, D=contains explicit `NaN` (quiet + signalling payloads) | `cfg_row22_nan_elements_disjoint` | [x] |
| 23 | `normalize` | A=`1..512`, B=disjoint, D=all elements identical (`sum = n*x²`, exact power-of-two cases) | `cfg_row23_identical_elements` | [x] |
| 24 | `normalize` | A=`1..512`, B=disjoint, D=exact small integers (unit vectors, `sum` exactly representable → `sqrtf` exact or not) | `cfg_row24_small_integers` | [x] |
| 25 | `normalize` | A=`1..256`, B=forward overlap `dest = src + k`, `k` in `1..8`, C=`sum>0` | `cfg_row25_forward_overlap` | [x] |
| 26 | `normalize` | A=`1..256`, B=backward overlap `src = dest + k`, `k` in `1..8`, C=`sum>0` | `cfg_row26_backward_overlap` | [x] |
| 27 | `normalize` | A=`1..256`, B=disjoint, E=`dest`/`src` misaligned by 1/2/3 floats (all 16 combinations) | `cfg_row27_misaligned_buffers` | [x] |
| 28 | `normalize` | A=vector-width boundaries exactly (`3,4,5,7,8,9,15,16,17,31,32,33,63,64,65`), B=disjoint, D=uniform | `cfg_row28_vector_width_boundaries` | [x] |
| 29 | `normalize` | A=vector-width boundaries exactly, B=in-place, D=uniform | `cfg_row29_vector_width_boundaries_inplace` | [x] |
| 30 | `normalize` | A=`1..64`, B=disjoint, D=one non-zero element among zeros at every position (sparse; exercises `sum>0` with exact single term) | `cfg_row30_sparse_single_nonzero` | [x] |
| 31 | `normalize` | A=`1..64`, B=disjoint, D=values chosen so `sum` is exactly `1.0f` (already-normalized input; `1.0f/sqrtf(1.0f) == 1.0f` → identity copy) | `cfg_row31_already_normalized` | [x] |
| 32 | `normalize` | A=`1..512`, B=disjoint, D=uniform, called **repeatedly on the same buffers** (idempotence / no hidden state; the C has no globals so two calls must be identical) | `cfg_row32_repeated_calls_no_state` | [x] |

No binary/driver executable exists on either side (see `SYMBOLS.md`), so there
is no stdout comparison row.

Only one feature configuration exists (no `[features]` in `Cargo.toml`), so the
whole table is run once per available build profile (debug + release `.so`).

## Cross-check: C compiler optimization level

The ground-truth build is the one `c_src/CMakeLists.txt` produces (no
`CMAKE_BUILD_TYPE`, so no `-O` flag). The whole suite was additionally run
against independently configured C builds:

| C build flags | result |
|---------------|--------|
| none (as `CMakeLists.txt` specifies) | 51/51 tests pass, bit-exact |
| `-O2` | 51/51 tests pass, bit-exact |
| `-O3 -march=native` | 1-ULP divergences (`err_row13`, `err_row15`, and the uniform rows) |

The `-O3 -march=native` divergence is **not** a translation defect: that build
enables FMA, and GCC's default `-ffp-contract=fast` then contracts
`sum += src[i]*src[i]` into a single `vfmadd` with one rounding step instead of
two. `objdump --disassemble=normalize` confirms 7 `vfmadd`/vector-FP
instructions in the `-march=native` object and 0 in the `-O2` object. The Rust
faithfully implements the two-rounding arithmetic the C *source* specifies, so
it matches every build that does not contract. Matching a contracting build
would require `f32::mul_add`, which would then mismatch the specified build.
