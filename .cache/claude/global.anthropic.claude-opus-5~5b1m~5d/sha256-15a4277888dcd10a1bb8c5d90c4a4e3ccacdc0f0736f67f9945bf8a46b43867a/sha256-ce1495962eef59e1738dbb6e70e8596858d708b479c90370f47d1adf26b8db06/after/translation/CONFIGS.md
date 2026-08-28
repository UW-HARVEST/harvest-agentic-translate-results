# CONFIGS.md — Phase A configuration-surface table (VALID inputs)

## How this was derived

The public surface is `c_src/include/lib.h`, which declares:

```c
typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;   /* size 4, align 1 */
typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;
void premultiply(cp_image_t *img);
```

`premultiply` is the **only** public entry point, and it is simultaneously the
lowest-level and the highest-level one — there is no convenience wrapper layered
over a lower primitive, so "exercise the low-level entry points too" is satisfied
by definition. There are **no** runtime options, modes, flags, or `#ifdef`s
anywhere in `c_src` (grep for `#if` returns nothing), so the configuration axes
are entirely **input shape**:

| axis | values the C code actually distinguishes | why (source evidence) |
|------|------------------------------------------|------------------------|
| `w` (width) | `0`; `1`; `2`; small odd; small even; large; negative; `±2^29`, `±2^30`, `2^28+1`, `INT_MIN`, `INT_MAX` | line 6 `stride = wrap32(w*4)`; line 8 sign of `limit` |
| `h` (height) | `0`; `1`; `2`; many; negative; `INT_MIN`, `INT_MAX` | line 8 `limit = wrap32(stride*h)` |
| sign combination | `(+,+)`, `(0,·)`, `(·,0)`, `(-,+)`, `(+,-)`, `(-,-)` | sign of `limit` decides run vs no-op |
| wrap class of `w*4` | no wrap / wraps to `0` / wraps to `INT_MIN` / wraps to other negative / wraps to other positive | `int` truncation on line 6 |
| alpha `a = data[i+3]` | `0`; `255`; `1..254`; all 256 | lines 9,13–15 scale factor |
| colour `r/g/b` | `0`; `255`; `1..254`; all 256 | lines 10–12, 16–18 |
| `(colour, alpha)` pair | the **complete 256×256 cross-product** | float round-then-truncate is value-dependent; only exhaustion proves it |
| `pix` alignment | 4-byte aligned; +1, +2, +3 byte offsets | accessed through `uint8_t*`, so unaligned is legal input |
| `pix` null-ness | non-null; null combined with `limit <= 0` | line 7 casts, line 9 derefs only if loop runs |
| channel written | `+0`,`+1`,`+2` written; `+3` (alpha) **preserved** | lines 16–18 store only three of four bytes |
| invocation count | once; twice (non-idempotent → must still agree) | premultiply is destructive in place |

Rows below are the cross-product of these axes, pruned to the combinations the
code treats differently.

Legend for "configuration": `w×h` gives the dimensions; `data` describes how the
pixel bytes are seeded. `rand(seed)` = deterministic `SplitMix64` PRNG, fixed
seed, **many** samples per row (see the per-row iteration counts in
`tests/differential.rs`).

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `premultiply` | `1×1`, data = all 65 536 `(colour, alpha)` combinations **exhaustively** (r=g=b=c, a) | [x] |
| 2 | `premultiply` | `1×1`, data = all 256 alphas × randomized distinct r,g,b (256 × 64 samples) | [x] |
| 3 | `premultiply` | `1×1`, `a = 0` (fully transparent) — colour must go to 0 | [x] |
| 4 | `premultiply` | `1×1`, `a = 255` (fully opaque) — colour must round-trip | [x] |
| 5 | `premultiply` | `1×1`, `a = 1` and `a = 254` (extreme non-saturated alphas), all 256 colours | [x] |
| 6 | `premultiply` | `1×1`, `r=g=b=255`, all 256 alphas (max colour) | [x] |
| 7 | `premultiply` | `1×1`, `r=g=b=0`, all 256 alphas (min colour) | [x] |
| 8 | `premultiply` | `1×N` single column, `N ∈ {1,2,3,7,64}`, data = `rand` | [x] |
| 9 | `premultiply` | `N×1` single row, `N ∈ {1,2,3,7,64}`, data = `rand` | [x] |
| 10 | `premultiply` | square `N×N`, `N ∈ {1,2,3,4,5,8,16,37}`, data = `rand` | [x] |
| 11 | `premultiply` | non-square `w×h`, randomized `w,h ∈ [1,40]`, data = `rand` (400 random shapes) | [x] |
| 12 | `premultiply` | odd width (`w` odd, tests no SIMD-tail assumption), `w ∈ {1,3,5,7,9,11,13}` × `h=3` | [x] |
| 13 | `premultiply` | even width `w ∈ {2,4,6,8,16}` × `h=3`, data = `rand` | [x] |
| 14 | `premultiply` | large-ish buffer `256×64` (16 384 px), data = `rand` | [x] |
| 15 | `premultiply` | `pix` misaligned by +1, +2, +3 bytes; `8×8`, data = `rand` | [x] |
| 16 | `premultiply` | data = all-zero buffer, `4×4` | [x] |
| 17 | `premultiply` | data = all-`0xFF` buffer, `4×4` | [x] |
| 18 | `premultiply` | data = alpha ramp `a = (index % 256)`, colours `0xFF`, `16×16` | [x] |
| 19 | `premultiply` | data = colour ramp, alpha fixed at each of `{0,1,127,128,254,255}`, `16×16` | [x] |
| 20 | `premultiply` | **double invocation** — call twice on the same buffer, `8×8`, `rand` (destructive, must still match) | [x] |
| 21 | `premultiply` | alpha-preservation: assert byte `+3` of every pixel is unchanged, `8×8`, `rand` | [x] |
| 22 | `premultiply` | write-extent: 64-byte canaries before/after buffer must be untouched, `8×8`, `rand` | [x] |
| 23 | `premultiply` | `w<0 && h<0` (row 10 of ERRORS.md) — `(-1,-1)`, `(-2,-3)`, `(-1,-4)`, `(-4,-4)`; loop RUNS, `rand` data | [x] |
| 24 | `premultiply` | `w = INT_MAX, h = -1` → 1 px; `w = INT_MAX, h = INT_MAX` → 1 px; `rand` data | [x] |
| 25 | `premultiply` | `w = 2^29+1, h = 2` → 2 px (wrap-to-positive), `rand` data | [x] |
| 26 | `premultiply` | `w = 2^28+1, h = 4` → 4 px (wrap-to-positive), `rand` data | [x] |
| 27 | `premultiply` | wrap-to-zero / wrap-to-negative no-ops with a **live buffer + canaries**: `w ∈ {2^30, -2^30, INT_MIN, ±2^29}` × `h ∈ {1,2,3,5}` | [x] |
| 28 | `premultiply` | `w=1` × `h ∈ {INT_MIN, INT_MAX}` and `w ∈ {INT_MIN,INT_MAX}` × `h=1`, live buffer + canaries | [x] |
| 29 | `premultiply` | randomized fuzz over the whole `(w, h)` **wrap** space: `w,h` drawn from a mix of small values, `±2^k` boundaries and `rand` `i32`s, executed only when the predicted `limit` fits the guarded buffer (2 000 samples) | [x] |
| 30 | `premultiply` | randomized fuzz over dimensions **and** data together: `w,h ∈ [0,24]`, fully random bytes, 2 000 samples | [x] |
| 31 | `premultiply` | `misaligned img` struct pointer (odd address) with `4×4` `rand` data | [x] |
| 32 | `premultiply` | struct field independence: `pix` pointing into the middle of a larger arena, `6×6` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the crate has exactly one configuration. `cargo metadata`
confirms the feature map is empty. The full matrix that Phase D must cover is
therefore:

| combo | command |
|-------|---------|
| default (= only) | `cargo test --release --offline` |
| explicit no-default | `cargo test --release --offline --no-default-features` |
| all features (empty set) | `cargo test --release --offline --all-features` |

`tests/features.rs::feature_matrix_is_exhaustive` asserts at test time that no
`[features]` section has appeared, so this claim cannot silently rot.
