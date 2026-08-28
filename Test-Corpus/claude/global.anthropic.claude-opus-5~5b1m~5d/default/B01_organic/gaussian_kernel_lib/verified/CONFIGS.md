# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

The complete public API is one function (`c_src/include/lib.h`):

```c
void gaussian_kernel(float *dest, int size, float radius);
```

There are **no** runtime option structs, no mode/flag arguments, no global
state, no `#ifdef`s and no Cargo features — so the configuration surface is the
cross product of the *input shapes* the C code actually branches on. Grepping
the source gives exactly two data-dependent branches plus the loop bounds:

| source line | branch | what selects it |
|-------------|--------|-----------------|
| `int r, hsize = size / 2;` + `for (r = -hsize; r <= hsize; r++)` | loop trip count `= (hsize < 0) ? 0 : 2*hsize + 1` | sign & parity of `size` (C division truncates toward zero) |
| `v = (((v) > (0)) ? (v) : (0));` | clamp taken / not taken (and the NaN case, where `>` is false) | `|x| = |r * (sigma/radius)|` vs `2.4000001` |
| `if (sum > 0.0f)` | normalise / skip normalisation (NaN `sum` ⇒ false) | whether any tap survived the clamp, i.e. whether `rs` is finite and the loop ran |
| `for (r = 0; r < size; r++) dest[r] *= isum;` | number of *scaled* elements (`size`) vs number of *stored* elements (`2*hsize+1`) | parity of `size` — for even `size` the last store is never scaled |

### Axis `S` — shape of `size`

| id | class | `hsize` | stores | normalised elems |
|----|-------|---------|--------|------------------|
| `S_ODD_SMALL` | odd, `3..=15` | `size/2` | `size` | `size` |
| `S_ONE` | `1` | `0` | `1` | `1` |
| `S_EVEN_SMALL` | even, `2..=16` | `size/2` | `size + 1` (**1 OOB**) | `size` |
| `S_ODD_LARGE` | odd, `63..=1025` | `size/2` | `size` | `size` |
| `S_EVEN_LARGE` | even, `64..=1024` | `size/2` | `size + 1` (**1 OOB**) | `size` |
| `S_ZERO` | `0` | `0` | `1` | `0` (unnormalised!) |
| `S_NEG_ONE` | `-1` | `0` (trunc-toward-zero) | `1` | `0` (unnormalised!) |
| `S_NEG` | `<= -2` | `<= -1` | `0` | `0` |
| `S_INT_MIN` | `INT_MIN` | `-2^30` | `0` | `0` |

### Axis `R` — shape of `radius` (drives `rs = sigma / radius`)

| id | class | `rs` | clamp behaviour | `sum > 0`? |
|----|-------|------|-----------------|------------|
| `R_TYPICAL` | uniform in `[0.25, 8]` | finite | mixed for large `size` | yes |
| `R_WIDE` | uniform in `[1e2, 1e6]` (very large) | tiny | never taken (all taps `> 0`) | yes |
| `R_NARROW` | uniform in `[1e-6, 1e-2]` but `rs` still finite | huge | taken for every `r != 0` | yes (centre tap only) |
| `R_THRESHOLD` | tuned so `|hsize * rs|` straddles `2.4000001` exactly | finite | boundary of the ternary | yes |
| `R_NEG` | negative mirror of `R_TYPICAL` | negative finite | same as `R_TYPICAL` (`x*x` symmetric) | yes |
| `R_POS_ZERO` | `+0.0` | `+inf` | `x = NaN` at `r=0`, `+inf` elsewhere ⇒ all clamped | **no** |
| `R_NEG_ZERO` | `-0.0` | `-inf` | all clamped | **no** |
| `R_POS_INF` | `+inf` | `+0.0` | never taken; every tap `= V0` | yes |
| `R_NEG_INF` | `-inf` | `-0.0` | never taken; every tap `= V0` | yes |
| `R_NAN` | NaN (quiet/signalling, both signs, several payloads) | NaN | all clamped (`NaN > 0` false) | **no** |
| `R_SUBNORMAL` | subnormal, small enough that `sigma/radius` overflows to `+inf` | `+inf` | all clamped | **no** |
| `R_EXTREME` | `FLT_MAX`, `FLT_MIN`, `-FLT_MAX`, `1.0`, `-1.0` | varies | varies | varies |
| `R_RANDOM_BITS` | uniform random 32-bit pattern reinterpreted as `float` | anything | anything | anything |

### Axis `F` — initial buffer contents (proves stores vs. read-modify-write)

`F_ZERO` (all `0x00000000`), `F_SENTINEL` (all `0x7f812345`, a NaN pattern the
code can never produce), `F_RANDOM` (random 32-bit patterns).

### Axis `P` — pointer shape

`P_BASE` (`dest` = buffer start), `P_OFFSET` (`dest` points 1/3/7 `f32`s into a
larger allocation, so the code may not rely on 16-byte base alignment),
`P_NULL` (only legal when the C never dereferences, i.e. `size <= -2`).

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_C0FFEE_u64`, ≥ 64 draws per row unless the row is a single exact
value) against **both** `.so`s, comparing the whole buffer — including the
pre-`dest` prefix and 16 `f32` of trailing guard padding — bit-for-bit.

| # | entry point(s) | configuration (options set + input shape) | test (`tests/phase_b_configs.rs`) | done |
|---|----------------|--------------------------------------------|-----------------------------------|------|
| C1 | `gaussian_kernel` | `S_ONE` × `R_TYPICAL` × `F_SENTINEL` × `P_BASE` — single tap, normalises to exactly `1.0f` | `c01_size_one_typical_radius` | [x] |
| C2 | `gaussian_kernel` | `S_ODD_SMALL` × `R_TYPICAL` × `F_SENTINEL` × `P_BASE` — exact-fit stores | `c02_odd_small_typical_radius` | [x] |
| C3 | `gaussian_kernel` | `S_EVEN_SMALL` × `R_TYPICAL` × `F_SENTINEL` × `P_BASE` — `size+1` stores, last one unnormalised | `c03_even_small_typical_radius_writes_one_past` | [x] |
| C4 | `gaussian_kernel` | `S_ODD_LARGE` × `R_TYPICAL` × `F_SENTINEL` × `P_BASE` | `c04_odd_large_typical_radius` | [x] |
| C5 | `gaussian_kernel` | `S_EVEN_LARGE` × `R_TYPICAL` × `F_SENTINEL` × `P_BASE` | `c05_even_large_typical_radius` | [x] |
| C6 | `gaussian_kernel` | `S_ODD_SMALL`+`S_EVEN_SMALL` × `R_WIDE` × `F_SENTINEL` — clamp never taken, all taps ≈ `V0` | `c06_wide_radius_clamp_never_taken` | [x] |
| C7 | `gaussian_kernel` | `S_ODD_LARGE`+`S_EVEN_LARGE` × `R_NARROW` — clamp taken for every off-centre tap ⇒ unit impulse at `hsize` | `c07_narrow_radius_clamp_always_taken` | [x] |
| C8 | `gaussian_kernel` | mixed sizes × `R_THRESHOLD` — ` | `c08_threshold_radius` | [x] |
| C9 | `gaussian_kernel` | mixed sizes × `R_NEG` — negative radius, `x*x` symmetric | `c09_negative_radius_mirror` | [x] |
| C10 | `gaussian_kernel` | mixed sizes × `R_EXTREME` (`FLT_MAX`, `FLT_MIN`, `±1.0`, `-FLT_MAX`) | `c10_extreme_finite_radii` | [x] |
| C11 | `gaussian_kernel` | mixed sizes × `R_POS_INF` — `rs = +0`, every tap `V0`, normalised (even `size` ⇒ `1/(size+1)`) | `c11_positive_infinity_radius` | [x] |
| C12 | `gaussian_kernel` | mixed sizes × `R_NEG_INF` — `rs = -0.0` | `c12_negative_infinity_radius` | [x] |
| C13 | `gaussian_kernel` | mixed sizes × `R_POS_ZERO` — `rs = +inf`, `sum == 0`, normalisation skipped | `c13_positive_zero_radius_skips_normalisation` | [x] |
| C14 | `gaussian_kernel` | mixed sizes × `R_NEG_ZERO` — `rs = -inf` | `c14_negative_zero_radius` | [x] |
| C15 | `gaussian_kernel` | mixed sizes × `R_NAN` (8 distinct NaN bit patterns incl. sNaN and sign-set) | `c15_nan_radii` | [x] |
| C16 | `gaussian_kernel` | mixed sizes × `R_SUBNORMAL` (incl. `0x00000001`, `0x007fffff`, negatives) — `rs` overflows to `±inf` | `c16_subnormal_radii` | [x] |
| C17 | `gaussian_kernel` | `S_ZERO` × all `R` classes — one unnormalised store | `c17_size_zero_unnormalised_single_store` | [x] |
| C18 | `gaussian_kernel` | `S_NEG_ONE` × all `R` classes — one unnormalised store (truncation toward zero) | `c18_size_minus_one_unnormalised_single_store` | [x] |
| C19 | `gaussian_kernel` | `S_NEG` (random in `-2..=-100000`) × all `R` classes × `F_RANDOM` — zero stores, buffer untouched | `c19_negative_size_no_stores` | [x] |
| C20 | `gaussian_kernel` | `S_INT_MIN` × all `R` classes — zero stores, no negation overflow | `c20_int_min_size` | [x] |
| C21 | `gaussian_kernel` | `S_NEG` × `P_NULL` — the only null-pointer configuration the C tolerates | `c21_null_dest_when_no_stores` | [x] |
| C22 | `gaussian_kernel` | all size classes × `R_TYPICAL` × `P_OFFSET` (offsets 1, 3, 7) — no base-alignment assumption | `c22_offset_dest_pointer` | [x] |
| C23 | `gaussian_kernel` | all size classes × `R_TYPICAL` × `F_ZERO` and `F_RANDOM` — pre-existing contents must not leak into the result (and must survive where the C does not store) | `c23_initial_buffer_contents` | [x] |
| C24 | `gaussian_kernel` | back-to-back invocation: same buffer called twice with different `(size, radius)` — verifies there is no hidden state and that the second call's partial overwrite matches | `c24_repeated_calls_share_no_state` | [x] |
| C25 | `gaussian_kernel` | full fuzz: `size` uniform in `-8..=64` × `R_RANDOM_BITS` (any 32-bit pattern) × random fill × random pointer offset, 20 000 draws | `c25_fuzz_small` | [x] |
| C26 | `gaussian_kernel` | full fuzz, large: `size` uniform in `1..=2048` × `R_RANDOM_BITS`, 2 000 draws | `c26_fuzz_large` | [x] |
| C27 | `gaussian_kernel` | exhaustive small sweep: every `size` in `-4..=40` × 24 radii per size drawn from every `R` class | `c27_exhaustive_small_size_sweep` | [x] |
| C28 | `gaussian_kernel` | boundary sizes `1, 2, 3` × radii tuned so `hsize*rs` lands exactly on `2.4000001`, `nextafter` below and above | `c28_clamp_boundary_exact` | [x] |
| C29 | `gaussian_kernel` | full matrix: every `representative_sizes()` value (incl. `INT_MIN`, 1024, 1025) x every `special_radii()` value | `c29_representative_size_times_special_radius_matrix` | [x] |

## Result

All 29 rows pass. Reproduce with:

```sh
cd translation
cargo build --offline                      # produces target/debug/libgaussian_kernel_lib.so
cargo test  --offline --test phase_b_configs
```

Evidence of the promised coverage (`-- --test-threads=1 --nocapture`):

```
c25_fuzz_small: 20000 randomized differential draws
c26_fuzz_large:  2000 randomized differential draws
[phase_b_configs] differential comparisons so far: 36590
```

i.e. Phase B alone performs **36 590** independent C-vs-Rust comparisons, each
of which compares the *whole* scratch buffer (bytes before `dest`, the payload,
and 16 `f32` of trailing guard) bit-for-bit.
