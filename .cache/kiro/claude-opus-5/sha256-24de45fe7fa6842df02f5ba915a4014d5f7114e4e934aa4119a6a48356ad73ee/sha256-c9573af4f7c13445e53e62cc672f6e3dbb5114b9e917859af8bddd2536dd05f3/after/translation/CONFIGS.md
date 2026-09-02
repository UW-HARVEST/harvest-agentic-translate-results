# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axis enumeration (from the source, not from guesses)

### 1. Runtime options / modes / flags

```sh
grep -nE '#if|#ifdef|#ifndef|#define|switch' c_src/src/lib.c c_src/include/lib.h   # -> NONE FOUND
```

There are **no** runtime options, no global state, no init/config function, no
`#ifdef` branches, and no `switch`. The public header declares exactly one
function and one struct. So the "options set" component of every row below is
constant: *(none)*.

### 2. Public entry points (the FULL set, including the lowest level)

| entry point | linkage | reachable across FFI? |
|---|---|---|
| `contrast_ratio(cb_rgb_255, cb_rgb_255)` | extern, `T` in `nm -D` | YES — tested |
| `cbContrastRatio(float×6)` | `static` | no — not in `nm -D`; only reachable *through* `contrast_ratio`, and only with the 256 values `n/255.f`. Exercised transitively. |
| `cbLuminance(float×3)` | `static` | no — same as above |

`contrast_ratio` **is** the lowest-level externally-callable entry point; the two
lower helpers have internal linkage and are deliberately not exported (see
`SYMBOLS.md`). Rows therefore drive the full composed pipeline
`contrast_ratio -> cbContrastRatio -> cbLuminance -> pow` end to end, which is
the only way the helpers can be reached at all.

### 3. Input shapes the C actually branches on

* **Per-channel transfer-function branch** (`cbLuminance`, 3× per color, 6× per
  call): `c > 0.04045` selects `pow((c+0.055)/1.055, 2.4)`, else `c/12.92`.
  With `c = n/255.f`, `n <= 10` → linear branch, `n >= 11` → `pow` branch.
  This gives **2^6 = 64** distinct branch-combination shapes per call.
* **High/Low swap branch** (`cbContrastRatio`): `if (High < Low)` — 2 shapes
  (`LumA >= LumB` no swap, `LumA < LumB` swap).
* **Degenerate divisor**: `Low == 0.0f` (a color is `{0,0,0}`) vs `Low > 0`.
  (The `Low == 0` shapes are the rows in `ERRORS.md`; listed here as the
  boundary of the valid domain.)
* **Value domain**: each channel is `unsigned char`, so 256 valid values;
  `256^6 = 2.8e14` total inputs. Boundary values: `0`, `1`, `10`, `11`, `254`,
  `255`.
* Element type / width / count / byte order / empty-one-many: **fixed by the
  ABI** — exactly 2 structs of exactly 3 `unsigned char`, native byte order.
  No arrays, no counts, no formats. No additional axis exists.

## Configuration table

"configuration" = *(no options — none exist)* + the input shape.
Each row is exercised with **many randomized inputs** from a fixed-seed PRNG
(deterministic SplitMix64, seed `0x5EED_C0FFEE`), asserting bit-identical `f32`
between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `contrast_ratio` | exhaustive grayscale: `A={n,n,n}`, `B={m,m,m}` for **all** 256×256 `(n,m)` pairs — covers both branch arms in all 6 channel slots simultaneously, both swap directions, and rows E1–E4 | [x] |
| C02 | `contrast_ratio` | all 64 per-channel branch combinations forced explicitly: for each of the 6 channel slots independently pick `low` (`n<=10`) or `high` (`n>=11`), randomized within each arm, 512 draws per combination | [x] |
| C03 | `contrast_ratio` | uniform random over the full domain, all 6 channels independent `0..=255`, 200 000 draws | [x] |
| C04 | `contrast_ratio` | swap path taken (`LumA < LumB`): 100 000 random pairs classified by a white-reference probe, plus all 256×256 ordered grayscale pairs with `n < m` | [x] |
| C05 | `contrast_ratio` | no-swap path (`LumA >= LumB`): same 100 000 random pairs, plus all ordered grayscale pairs with `n > m` | [x] |
| C06 | `contrast_ratio` | `A == B` identical colors, all 256 grayscale + 4096 random identical pairs (row E4) | [x] |
| C07 | `contrast_ratio` | both colors confined to the **linear** arm only: all 11^3 = 1331 colors in `0..=10`^3 crossed against 7 fixed partners in both orders, plus 50 000 random draws inside the box | [x] |
| C08 | `contrast_ratio` | both colors confined to the **pow** arm only: every channel in `11..=255`, 50 000 random draws | [x] |
| C09 | `contrast_ratio` | boundary values only: full 216 × 216 = 46 656 cross product of `{0,1,10,11,254,255}^3` against itself | [x] |
| C10 | `contrast_ratio` | one channel varied over all 256 values while the other 5 are pinned to a random constant — for each of the 6 slots, 64 random pinnings (value-dependent path coverage) | [x] |
| C11 | `contrast_ratio` | `Low == 0` divisor via non-swap path: `B={0,0,0}`, `A` random non-black (row E1) | [x] |
| C12 | `contrast_ratio` | `Low == 0` divisor via swap path: `A={0,0,0}`, `B` random non-black (row E2) | [x] |
| C13 | `contrast_ratio` | `0/0`: both `{0,0,0}` (row E3) | [x] |
| C14 | `contrast_ratio` | extremes: `{0,0,0}` vs `{255,255,255}` and the reverse — max ratio, both swap directions | [x] |
| C15 | `contrast_ratio` | single-channel-only colors (`{n,0,0}`, `{0,n,0}`, `{0,0,n}`) crossed against each other for all `n in 0..=255` — isolates each of the 0.2126/0.7152/0.0722 weights and hits `Low==0` when the varied channel is 0 | [x] |
| C16 | `contrast_ratio` | ABI upper-bits garbage: 20 000 random color pairs invoked through an `extern "C" fn(u64,u64)->f32` view of the symbol with random garbage in bits 24..63 of each argument register (row E7) | [x] |
| Z01 | `contrast_ratio` | **whole-domain exhaustion**: all 2^24 colors against two fixed references (white, and `{11,11,11}` on the `pow` boundary) = 33 554 432 differential calls per side | [x] |

### Row-to-test mapping

| row | test in `translation/tests/differential.rs` |
|---|---|
| C01 | `c01_exhaustive_grayscale_pairs` |
| C02 | `c02_all_channel_branch_combinations` |
| C03 | `c03_uniform_random_full_domain` |
| C04, C05 | `c04_c05_both_swap_directions` |
| C06 | `c06_identical_colors` |
| C07 | `c07_linear_arm_only` |
| C08 | `c08_pow_arm_only` |
| C09 | `c09_boundary_value_cross_product` |
| C10 | `c10_single_channel_sweeps` |
| C11 | `e01_divide_by_zero_no_swap` |
| C12 | `e02_divide_by_zero_swap_path` |
| C13 | `e03_zero_over_zero_nan` |
| C14 | `c14_extremes` |
| C15 | `c15_single_channel_colors` |
| C16 | `c16_e7_abi_upper_bit_garbage` |
| Z01 | `z01_exhaustive_all_16m_colors` |
| harness self-check | `z02_harness_loads_two_distinct_libraries`, `d01_symbol_parity` |

### Why this is complete coverage of the valid domain

The full input domain is `256^6 ≈ 2.8e14` pairs, which cannot be enumerated. But
the C computation factors exactly as `f(Lum(A), Lum(B))` where
`f(x, y) = max(x, y) / min(x, y)`:

* Z01 enumerates **every** one of the 2^24 colors, so every reachable `Lum`
  value — and therefore every `pow` argument, every branch-arm selection, and
  every narrowing cast — is compared bit-for-bit on both sides.
* `f` itself is one `<` and one `divss`. C01 exhausts it over all 256×256
  grayscale pairs, C04/C05 exhaust both branch directions, and C11–C13 exhaust
  its three degenerate cases (`Low == 0` in either order, `0/0`).

Together these cover every distinct code path and every reachable intermediate
value, not just a sample.


### Feature combinations

`translation/Cargo.toml` has no `[features]` table → exactly one combination.
Rows C01–C16 and Z01 are run under all four build configurations produced by
`run_matrix.sh` (`{debug, release} × {default, --no-default-features}`), all
passing 21/21. Every randomized row uses the fixed SplitMix64 seed
`0x5EED_C0FF_EE00_0001` xor'd with the row number, so runs are reproducible.

### Harness caveat worth knowing

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact. The
harness therefore builds the Rust `.so` itself into `target/harness/<profile>/`
(separate `CARGO_TARGET_DIR`, so it cannot deadlock on the outer `cargo test`
build lock) and asserts the object is newer than `src/lib.rs`. Without this the
whole suite passes vacuously against a stale object — see the mutation-testing
section of `ERRORS.md`.

