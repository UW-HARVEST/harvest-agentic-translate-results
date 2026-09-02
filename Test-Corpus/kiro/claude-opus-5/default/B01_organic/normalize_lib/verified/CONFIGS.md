# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

## Axes derived from the C source

There is exactly one public entry point, and it is also the lowest-level one:

```c
void normalize(float *dest, const float *src, int size);   /* c_src/include/lib.h */
```

There are no convenience wrappers, no init/teardown, no opaque context, and no
higher layer to compose — the whole library is this one call, so "driving it the
way a real consumer does" *is* calling `normalize` directly.

Runtime options/modes/flags: **none**. Verified mechanically:

```sh
grep -nE '#ifdef|#if |switch|enum|typedef|extern' -r c_src/src c_src/include   # no match
```

So the configuration axes are the *input shapes* the code branches on. The C
has exactly three branch points, and they define the axes:

* `A. size` — controls both loop trip counts (`i < size`) and the `memset`
  length (`size * sizeof(float)`).
  Values: `0`, `1`, `2`, `3`, `4`, `5`, `7`, `8`, `15`, `16`, `17`, `31`, `33`,
  `64`, `1000`, `4096`, `65536` (odd/even, powers of two ±1, "empty / one /
  many"), plus the negative and `INT_MIN`/`INT_MAX` cases which live in
  `ERRORS.md`.
* `B. pointer relationship` — the `dest != src` test, and the read-then-write
  ordering of the second loop.
  Values: `disjoint` · `identical (dest == src, in-place)` ·
  `forward overlap (dest == src + 1)` · `backward overlap (dest == src - 1)` ·
  `partial overlap (dest == src + size/2)`.
* `C. the class of `sum`` — the `sum > 0.0f` test.
  Values: `positive normal` · `exactly +0.0 (all inputs zero)` ·
  `exactly +0.0 by underflow (non-zero inputs whose squares round to 0)` ·
  `positive denormal` · `+inf (overflow)` · `NaN (NaN input)` ·
  `+inf via ±inf input`.
* `D. element value population` — value-dependent rounding in the accumulation
  and in `src[i] * sum`.
  Values: `uniform [-1,1]` · `wide exponent range` · `all denormal` ·
  `mixed denormal + normal` · `near FLT_MAX` · `±0.0 mixed` ·
  `±inf sprinkled` · `NaN sprinkled` · `exact powers of two` ·
  `sum contrived to be exactly 1.0`.

Rows below are the cross-product **pruned to the combinations the C actually
distinguishes**: axis C × axis B is the real decision table (which of the three
code paths runs, and whether the write is even performed), and axes A × D are
swept randomly inside every row.

Every row is driven with **many randomized inputs** from a seeded xorshift64\*
PRNG (`SEED = 0x9E3779B97F4A7C15`, fixed → reproducible), not a single
hand-picked value. Each iteration re-randomises `size` (from axis A), the
element population (from axis D) and the buffer offsets, runs the C `.so` and
the Rust `.so` on **identical** copies of the same buffer, and compares the
*entire* buffer bit-for-bit (`f32::to_bits`), including 8-element guard bands
before and after the `dest` region so that any over- or under-write is caught.

## The table

| # | entry point(s) | configuration (options set + input shape) | iters | test | [x] |
|---|----------------|--------------------------------------------|-------|------|-----|
| C1 | `normalize` | disjoint; `sum` positive normal; uniform `[-1,1]`; sizes swept `1..=64` and `1000/4096` | 2000 | `cfg_c1_disjoint_uniform` | [x] |
| C2 | `normalize` | disjoint; `sum` positive normal; **wide exponent range** (`2^-60 … 2^+60`, random signs) | 2000 | `cfg_c2_disjoint_wide_exponent` | [x] |
| C3 | `normalize` | disjoint; exact powers of two only (exact arithmetic, exposes any rounding-mode/order difference) | 1000 | `cfg_c3_disjoint_powers_of_two` | [x] |
| C4 | `normalize` | disjoint; population contrived so `sum == 1.0f` exactly → scale `== 1.0f` → `dest` must equal `src` bitwise (incl. `-0.0`) | 500 | `cfg_c4_disjoint_sum_exactly_one` | [x] |
| C5 | `normalize` | **in-place** (`dest == src`); `sum` positive normal; wide exponent range | 2000 | `cfg_c5_inplace_normal` | [x] |
| C6 | `normalize` | **forward overlap** `dest == src + 1` (write of `dest[i]` clobbers `src[i+1]` before it is read) | 2000 | `cfg_c6_overlap_forward_1` | [x] |
| C7 | `normalize` | **backward overlap** `dest == src - 1` | 2000 | `cfg_c7_overlap_backward_1` | [x] |
| C8 | `normalize` | **partial overlap** `dest == src + size/2` | 2000 | `cfg_c8_overlap_half` | [x] |
| C9 | `normalize` | disjoint; `size == 1` (single element, `dest = ±1.0` exactly); wide exponent range incl. denormals | 1000 | `cfg_c9_size_one` | [x] |
| C10 | `normalize` | disjoint; `size == 0` (empty) → `memset` of 0 bytes, `dest` guard bands must be pristine | 200 | `cfg_c10_size_zero` | [x] |
| C11 | `normalize` | disjoint; **all elements zero**, random mix of `+0.0` / `-0.0` → `sum == +0.0` → zero-fill path | 1000 | `cfg_c11_all_zeros_disjoint` | [x] |
| C12 | `normalize` | **in-place**; all elements zero → `dest == src` so *nothing* is written; `-0.0` must survive | 1000 | `cfg_c12_all_zeros_inplace` | [x] |
| C13 | `normalize` | disjoint; **all denormal** inputs whose squares underflow to `+0.0` → `sum == +0.0` → zero-fill despite non-zero input | 1000 | `cfg_c13_denormal_underflow_disjoint` | [x] |
| C14 | `normalize` | **in-place**; same underflowing denormal population → no write at all | 1000 | `cfg_c14_denormal_underflow_inplace` | [x] |
| C15 | `normalize` | disjoint; `sum` lands in the **denormal** range → `1.0f/sqrtf(denormal)` huge, results may overflow to `±inf` | 1000 | `cfg_c15_denormal_sum` | [x] |
| C16 | `normalize` | disjoint; **mixed denormal + normal** elements (accumulation order matters) | 2000 | `cfg_c16_mixed_denormal_normal` | [x] |
| C17 | `normalize` | disjoint; magnitudes **near `FLT_MAX`** → `sum` overflows to `+inf` → scale `+0.0` → `dest[i] = ±0.0` | 1000 | `cfg_c17_sum_overflow_inf` | [x] |
| C18 | `normalize` | **in-place**; magnitudes near `FLT_MAX` (`sum == +inf`, so the *writing* branch still runs in-place) | 1000 | `cfg_c18_sum_overflow_inf_inplace` | [x] |
| C19 | `normalize` | disjoint; `±inf` sprinkled among finite elements → `sum == +inf` → `inf*0.0 = NaN`, finite`*0.0 = ±0.0` | 1000 | `cfg_c19_inf_elements_disjoint` | [x] |
| C20 | `normalize` | disjoint; **NaN sprinkled** (random payloads/signs, quiet and signalling) → `sum` NaN → `sum > 0` false → zero-fill | 1000 | `cfg_c20_nan_elements_disjoint` | [x] |
| C21 | `normalize` | **in-place**; NaN sprinkled → nothing written, exact NaN payload bits preserved | 1000 | `cfg_c21_nan_elements_inplace` | [x] |
| C22 | `normalize` | **overlapping** (`dest == src + 1`) with the zero-fill path (`sum == +0.0`): `dest != src` is true, so `memset` runs over a region that overlaps `src` | 1000 | `cfg_c22_overlap_zero_fill` | [x] |
| C23 | `normalize` | disjoint; **fully random 32-bit bit patterns** reinterpreted as `f32` (every class at once: normals, denormals, `±0`, `±inf`, NaN) — the unconstrained fuzz row | 4000 | `cfg_c23_disjoint_random_bits` | [x] |
| C24 | `normalize` | **in-place**; fully random 32-bit bit patterns | 4000 | `cfg_c24_inplace_random_bits` | [x] |
| C25 | `normalize` | **overlap** (random offset delta in `-4..=4`); fully random 32-bit bit patterns | 4000 | `cfg_c25_overlap_random_bits` | [x] |
| C26 | `normalize` | disjoint; **large** buffer `size == 65536` (long accumulation chain; catches any vectorised/reassociated summation in the Rust) | 20 | `cfg_c26_large_size` | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so there is a single
configuration. `run_all.sh` still runs the suite under both `cargo test` and
`cargo test --no-default-features` (and the release profile, which is where
`panic = "abort"` applies) so that all buildable configurations are covered.

## Results

All 26 rows pass across their randomized inputs (≈45 000 differential calls in
total), in both the dev and the release profile and under both feature
configurations:

```
cargo test --test configs   ->  27 passed; 0 failed
```

(26 rows + `harness_loads_two_distinct_shared_objects`, which asserts the two
`.so` files really are distinct files and that both resolve `normalize`.)

No divergence was found on any valid-input row — the single divergence in this
translation was on the NULL-pointer error paths and is documented in
`ERRORS.md`. The suite's ability to detect divergence on these rows is
demonstrated by the mutation table in `ERRORS.md` (accumulating in `f64`, or
accumulating in reverse index order, each break 15 of these tests).
