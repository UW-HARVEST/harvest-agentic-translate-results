# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from `c_src/src/lib.c`. The library has **one** public
entry point and **no** runtime flags, modes, `#ifdef`s, or state — so the
configuration surface is the cross-product of the two input axes the code
actually branches on, times the data shapes it special-cases.

## Public entry points (full set, from `c_src/include/lib.h`)

| entry point | signature | note |
|---|---|---|
| `gaussian_kernel` | `void gaussian_kernel(float *dest, int size, float radius)` | the only exported symbol; there is no higher-level wrapper and no lower-level helper, so this *is* the lowest-level entry point |

## Axes the C actually branches on

Grep of every branch in the C:

```
lib.c:10   int r, hsize = size / 2;              -> truncating div; sign of size
lib.c:12   rs = sigma / radius;                  -> radius == 0 / inf / NaN / denormal
lib.c:15   for (r = -hsize; r <= hsize; r++)     -> AXIS 1: size (parity + sign + magnitude)
lib.c:17   float v = (1.0f/expf(x*x)) - s2;      -> AXIS 2: radius (scale regime)
lib.c:18   v = ((v) > (0)) ? (v) : (0);          -> BRANCH: per-element clamp
lib.c:23   if (sum > 0.0f) {                     -> BRANCH: normalize or not
lib.c:25   for (r = 0; r < size; r++)            -> writes 2*hsize+1 but normalizes size
```

- **Axis 1 — `size`**: sign (`<= -2`, `-1`, `0`, `> 0`), parity (odd → exactly
  `size` writes; even → `size + 1` writes, i.e. one past the end), magnitude
  (1, 2, 3, small, large).
- **Axis 2 — `radius` regime**, which selects how many elements survive the
  `v > 0` clamp and therefore whether `sum > 0.0f`:
  - `R_TINY`: `rs` so large that every `r != 0` overflows → only the centre
    survives (or nothing does).
  - `R_SMALL`: partial clamping — tails are zero, centre band positive.
  - `R_MID` / `R_LARGE`: nothing (or almost nothing) clamped.
  - `R_HUGE`: `rs` ≈ 0 → all elements equal.
  - `R_NEG`: negative radius (sign cancels in `x*x`).
  - degenerate: `±0.0`, `±inf`, `NaN`, subnormal, `f32::MAX`.
- **Interaction that matters**: the *window size* (`2*hsize+1`) times the
  *radius scale* determines the clamp pattern and whether `sum` underflows to
  `0.0f`, so both axes must be crossed, not tested independently.

Every row is driven with **many randomized inputs** (fixed seed, xorshift PRNG
in `tests/common/mod.rs`) over the free parameter of the row, and the full
written region **plus one guard element past it** is compared bitwise between
the C `.so` and the Rust `.so`.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `gaussian_kernel` | `size = 1` (min positive, `hsize=0`, 1 write) × randomized positive `radius` in `[1e-3, 1e3]` | [x] |
| 2 | `gaussian_kernel` | `size = 2` (min even → 3 writes, OOB-by-one) × randomized positive `radius` | [x] |
| 3 | `gaussian_kernel` | `size = 3` (min odd > 1, 3 writes) × randomized positive `radius` | [x] |
| 4 | `gaussian_kernel` | `size = 4` (even, 5 writes) × randomized positive `radius` | [x] |
| 5 | `gaussian_kernel` | `size = 5` (odd) × randomized positive `radius` | [x] |
| 6 | `gaussian_kernel` | randomized **odd** `size` in `[7, 129]` × randomized positive `radius` in `[1e-3, 1e3]` | [x] |
| 7 | `gaussian_kernel` | randomized **even** `size` in `[6, 128]` × randomized positive `radius` (verifies the `size+1`-th write every time) | [x] |
| 8 | `gaussian_kernel` | large `size` (`1023`, `1024`, `4095`, `4096`) × randomized positive `radius` | [x] |
| 9 | `gaussian_kernel` | `R_TINY`: randomized `radius` in `[1e-38, 1e-6]` × randomized odd+even `size` (only centre survives clamp, or nothing) | [x] |
| 10 | `gaussian_kernel` | `R_SMALL`: randomized `radius` in `[0.05, 0.9]` × randomized `size` (partial clamp: zero tails, positive band) | [x] |
| 11 | `gaussian_kernel` | `R_MID`: randomized `radius` in `[0.9, 8.0]` × randomized `size` (mixed clamp near the boundary) | [x] |
| 12 | `gaussian_kernel` | `R_LARGE`: randomized `radius` in `[8.0, 1e6]` × randomized `size` (nothing clamped) | [x] |
| 13 | `gaussian_kernel` | `R_HUGE`: randomized `radius` in `[1e30, f32::MAX]` × randomized `size` (`rs` → ~0, all elements equal) | [x] |
| 14 | `gaussian_kernel` | `R_NEG`: randomized **negative** `radius` (mirrors of rows 9–13) × randomized `size` | [x] |
| 15 | `gaussian_kernel` | fully randomized bit-pattern `radius` (arbitrary finite `f32`, incl. subnormals) × fully randomized `size` in `[0, 96]` — property fuzz over the whole cross-product | [x] |
| 16 | `gaussian_kernel` | `size` from randomized bit patterns restricted to `[-4, 96]` × `R_MID` (covers the `size<=-2`, `-1`, `0` guard rows as *valid* inputs too) | [x] |
| 17 | `gaussian_kernel` | pre-filled `dest` buffer with a non-zero poison pattern, `size` even (checks which bytes are left untouched vs. overwritten, incl. the OOB element and everything past it) | [x] |
| 18 | `gaussian_kernel` | repeated in-place invocation on the same buffer (call twice with different `radius`) — verifies the function is stateless and fully overwrites | [x] |
| 19 | `gaussian_kernel` | `radius` exactly at clamp boundary: values chosen so `1/expf(x*x) == s2` to within 1 ULP for some `r` (searched numerically) × odd/even `size` | [x] |
| 20 | `gaussian_kernel` | unaligned-ish / offset destination: kernel written into the middle of a larger allocation (`dest = base.add(k)`) × randomized `size`, `radius` | [x] |

## Binary / driver

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED ...)`; there is
no `add_executable`. The Rust `Cargo.toml` declares `crate-type = ["cdylib"]`
and no `[[bin]]`. **No binary driver exists, so the stdout-comparison item is
not applicable.**

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section and no optional
dependencies, so the only build configuration is the default one. Verified by
`scripts/feature_matrix.sh`, which enumerates features from `Cargo.toml` and
runs the full suite for each combination (default, `--no-default-features`).

## Row → test mapping

Every row above is implemented in `tests/valid_paths.rs` as `rowNN_*`, driven
through both `.so`s via `libloading` (the Rust function is never called
directly). All 20 rows pass across their randomized inputs under every
configuration; see `scripts/verify.sh`.

Additional coverage beyond the table, in `tests/heavy_sweep.rs`:

| test | coverage |
|---|---|
| `sweep_radius_bit_space_strided` | 42,524,429 distinct `radius` bit patterns (stride 101 over the full 2^32 space, so every exponent field and a dense mantissa grid, incl. inf/NaN encodings) × sizes {1, 2, 3, 4, 7} = ~212M differential calls |
| `sweep_size_space_dense` | every `size` in `[-8, 2048]` × 12 radius regimes |
| `fuzz_joint_random_bits` | 200,000 random (`size`, `radius`-bit-pattern) pairs |
| `fuzz_null_dest_no_write_regime` | 200,000 random `size <= -2` values with a NULL destination |

Set `SWEEP_STRIDE=1` for a fully exhaustive 2^32 radius sweep (~35 min).
