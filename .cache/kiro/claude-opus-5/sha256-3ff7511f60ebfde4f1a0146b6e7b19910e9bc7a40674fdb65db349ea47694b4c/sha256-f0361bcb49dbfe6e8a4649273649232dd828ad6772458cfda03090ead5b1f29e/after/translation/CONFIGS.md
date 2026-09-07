# CONFIGS.md — Phase B configuration-surface table

## Axes derived from the C source

`c_src/include/lib.h` declares the **complete** public API — a single entry
point, and it is already the lowest-level one (there is no convenience wrapper
layered on top of anything):

```c
float pow43(int x);
```

There are no runtime options, no init/context object, no flags, no modes, no
`#ifdef`, no byte-order or element-type parameters, and no `switch`. Everything
the code distinguishes is a function of the single `int` argument. The axes are
therefore exactly the branches the C body takes:

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| **A. code path** | `x < 129` (direct table lookup, early return) / `129 <= x < 1024` (`mult = 16`, `x <<= 3`, then interpolate) / `x >= 1024` (`mult = 256`, no shift, then interpolate) | `if (x < 129)` line 37, `if (x < 1024)` line 40 |
| **B. `mult`** | `256` (default, taken by `x >= 1024`) / `16` (taken by `129 <= x < 1024`) | line 36 / line 41 |
| **C. `sign`** | `0` / `64`, from `sign = 2 * x & 64` i.e. bit 5 of the (post-shift) `x`. On path B `x` was shifted by 3, so `sign = (16*x_orig) & 64` ⇒ selected by bit 2 of the original `x`. On path C it is bit 5 of `x`. | line 43 |
| **D. `frac` numerator sign** | `(x & 63) - sign` is `>= 0` (when `sign == 0`) or `<= 0` (when `sign == 64`, since `x & 63 <= 63`); it is exactly `0` when `(x & 63) == sign` | line 44 |
| **E. table half** | negative-valued entries `g_pow43[0..=15]` (reached only by `x ∈ [-16,-1]`) / `g_pow43[16] == 0` / positive entries `g_pow43[17..=144]` | table decl lines 3–35 |
| **F. input shape / boundary values** | domain min `-16`; negatives; `0`; small positives; branch boundaries `128`/`129` and `1023`/`1024`; multiples of 64 (`frac == 0`); `x ≡ 63 (mod 64)`; domain max `8223` | derived from A–E |

Implicit valid domain (see `ERRORS.md`): `x ∈ [-16, 8223]`.

## Rows = the pruned cross-product the C actually treats differently

Every row is exercised against **both** `.so`s through `libloading`, comparing
the returned `float` **bit-for-bit** (`f32::to_bits`), with many randomized
inputs per row from a fixed-seed (`0x2545F4914F6CDD1D`) xorshift64* PRNG.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pow43` | path A, negative table half: `x ∈ [-16, -1]` (exhaustive, all 16) | [x] |
| 2 | `pow43` | path A, `x == 0` (the `g_pow43[16] == 0` entry; checks signed zero) | [x] |
| 3 | `pow43` | path A, positive: `x ∈ [1, 128]` (exhaustive, all 128) | [x] |
| 4 | `pow43` | path A, boundary: `x == -16` (first valid index, 0) | [x] |
| 5 | `pow43` | path A, boundary: `x == 128` (last input taking the early return) | [x] |
| 6 | `pow43` | path B (`mult == 16`, `x <<= 3`), `sign == 0` ⇒ `x & 4 == 0`; randomized over `[129, 1023]` | [x] |
| 7 | `pow43` | path B, `sign == 64` ⇒ `x & 4 != 0`; randomized over `[129, 1023]` | [x] |
| 8 | `pow43` | path B, boundary: `x == 129` (first input on the interpolating path) | [x] |
| 9 | `pow43` | path B, boundary: `x == 1023` (post-shift `8184`, max table index 144) | [x] |
| 10 | `pow43` | path B, `frac == 0`: `x` a multiple of 8 with `x & 4 == 0` ⇒ `(x<<3) & 63 == 0` and `sign == 0` | [x] |
| 11 | `pow43` | path B, `frac` maximally negative: `x & 7 == 0`… `x & 63` combinations with `sign == 64` (numerator `< 0`) | [x] |
| 12 | `pow43` | path B, exhaustive over the entire interval `[129, 1023]` | [x] |
| 13 | `pow43` | path C (`mult == 256`, no shift), `sign == 0` ⇒ `x & 32 == 0`; randomized over `[1024, 8223]` | [x] |
| 14 | `pow43` | path C, `sign == 64` ⇒ `x & 32 != 0`; randomized over `[1024, 8223]` | [x] |
| 15 | `pow43` | path C, boundary: `x == 1024` (first input with `mult == 256` and no shift) | [x] |
| 16 | `pow43` | path C, `frac == 0`: `x` a multiple of 64 (`x & 63 == 0`, `sign == 0`) | [x] |
| 17 | `pow43` | path C, `x ≡ 63 (mod 64)` (`sign == 64`, numerator `63 - 64 = -1`, denominator `+64`) | [x] |
| 18 | `pow43` | path C, `x ≡ 32 (mod 64)` (`sign == 64`, numerator `32 - 64 = -32`, the largest-magnitude negative `frac`) | [x] |
| 19 | `pow43` | path C, `x ≡ 31 (mod 64)` (`sign == 0`, numerator `31`, the largest positive `frac` before the sign flips) | [x] |
| 20 | `pow43` | path C, boundary: `x == 8223` (largest in-domain input, table index 144) | [x] |
| 21 | `pow43` | path C, exhaustive over the entire interval `[1024, 8223]` | [x] |
| 22 | `pow43` | **all paths**, exhaustive sweep over the complete defined domain `x ∈ [-16, 8223]` (8240 inputs, bit-exact) | [x] |
| 23 | `pow43` | monotone/consistency cross-check: the two branch boundaries `128→129` and `1023→1024` must agree between C and Rust on both sides simultaneously | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED src/lib.c)` — no
`add_executable`. `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` — no `[[bin]]`. **There is no driver binary**, so
the "compare C and Rust stdout byte-for-byte" clause is not applicable.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**. The complete set of
buildable feature combinations is the single empty set; `--no-default-features`
is equivalent to the default build. All rows above were run under both
invocations (see the sweep script output in the verification log).
