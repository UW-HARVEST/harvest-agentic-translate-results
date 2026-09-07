# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## Mechanical derivation of the axes

Public API surface (`c_src/include/lib.h`), complete:

```c
typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;   /* size 4, align 1 */
typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;
void premultiply(cp_image_t *img);
```

There is exactly **one** public entry point and it is simultaneously the
lowest-level and the highest-level one — there is no convenience wrapper to hide
behind, so "exercise the low-level entry points directly" is satisfied by
calling `premultiply` through the `.so` export.

Runtime options / flags: **none** (no setters, no context struct, no globals, no
`#ifdef`). The configuration space is therefore entirely the space of *input
shapes* plus *pixel data shapes*. The axes below are read off the four
expressions the C actually branches on:

* **Axis W** — `img->w`: `0`, `1`, small (`2..8`), non-trivial (`>16`), large,
  `stride`-overflowing, negative. (`stride = w * 4`.)
* **Axis H** — `img->h`: `0`, `1`, small, many, negative. (`bound = stride * h`.)
* **Axis A** — alpha channel value: `0`, `1`, mid, `254`, `255`, mixed per pixel,
  and **exhaustively all 256 values** (the only value-dependent arithmetic in the
  library is `(uint8_t)((c/255.0f)*(a/255.0f)*255.0f)`, so the *whole* function
  is a 256x256 lookup table and can be verified exhaustively).
* **Axis C** — colour channel values `r`,`g`,`b`: `0`, `255`, mid, per-channel
  distinct (proves no channel index is swapped: C writes `[0],[1],[2]` from
  `r,g,b` and reads alpha from `[+3]`, and **never writes `[+3]`**).
* **Axis P** — `pix` pointer provenance: heap-allocated exact-size buffer,
  over-allocated buffer with guard/pad bytes after the region (to detect a
  one-pixel-too-far or one-pixel-too-short loop bound), unaligned-by-1 pointer
  (legal: `cp_pixel_t` has alignment 1), `NULL` when the bound is 0.
* **Axis R** — repeat/idempotency: applying `premultiply` twice must produce the
  same second-application result in both implementations (catches state leaks;
  the C keeps no state).

Cross-product pruned to the combinations the C code actually distinguishes:

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `premultiply` | `w=1, h=1`, single pixel, randomized `r,g,b,a` (fixed-seed LCG, 4096 draws) — smallest non-empty shape | [x] |
| 2 | `premultiply` | `w=1, h=1`, **exhaustive** over all `a ∈ 0..=255` x all `c ∈ 0..=255` applied to `r=g=b=c` (65 536 cases) — full value-dependent rounding table | [x] |
| 3 | `premultiply` | `w=1, h=1`, per-channel distinct values `r≠g≠b`, exhaustive `a ∈ 0..=255` — proves channel ordering/indexing `[0]=r,[1]=g,[2]=b`, alpha read from `[3]` | [x] |
| 4 | `premultiply` | `a = 0` for every pixel, `w=7, h=5`, randomized colours — the all-transparent path (result must be all-zero RGB, alpha preserved) | [x] |
| 5 | `premultiply` | `a = 255` for every pixel, `w=7, h=5`, randomized colours — the opaque path (result must be *bit-identical* to input; truncation must not lose 1 LSB) | [x] |
| 6 | `premultiply` | `a = 1` and `a = 254` (the two near-boundary alphas) for every pixel, `w=9, h=3`, randomized colours | [x] |
| 7 | `premultiply` | `r=g=b=255, a` randomized, `w=16, h=16` — colour channels at max | [x] |
| 8 | `premultiply` | `r=g=b=0, a` randomized, `w=16, h=16` — colour channels at min | [x] |
| 9 | `premultiply` | `w=1, h=N` (`N ∈ {1,2,3,17,64}`), fully randomized pixels — single-column shape, `stride=4` | [x] |
| 10 | `premultiply` | `w=N, h=1` (`N ∈ {1,2,3,17,64}`), fully randomized pixels — single-row shape | [x] |
| 11 | `premultiply` | `w=h=N` square (`N ∈ {2,3,4,5,8,13,16,31,32}`), fully randomized pixels — many shapes, exercises the `stride*h` byte-counted bound | [x] |
| 12 | `premultiply` | rectangular `w≠h` (`(3,7),(7,3),(1,255),(255,1),(17,19),(64,3)`), fully randomized pixels | [x] |
| 13 | `premultiply` | larger image `w=131, h=97` (12 707 pixels), fully randomized — beyond any plausible unrolled/vectorized tail boundary | [x] |
| 14 | `premultiply` | randomized `w,h` in `1..=40` with randomized pixels, 300 independent property-style iterations (fixed seed) — general shape x value fuzz | [x] |
| 15 | `premultiply` | **padded buffer**: allocate `w*h + 8` pixels, set `img->w/h` to the logical size, fill the 8 trailing guard pixels with a known pattern; assert C and Rust agree on the *entire* buffer incl. guards — detects off-by-one in the loop bound in either direction | [x] |
| 16 | `premultiply` | **unaligned `pix`**: buffer offset by 1 byte from an allocation (legal, `align_of::<cp_pixel_t>() == 1`), `w=11, h=5`, randomized pixels | [x] |
| 17 | `premultiply` | **idempotency / repeat application**: call `premultiply` twice on the same buffer in both libs, `w=13, h=11`, randomized pixels; assert byte-equality after each call | [x] |
| 18 | `premultiply` | **independent-instance / no-shared-state**: interleave calls on two different images (different shapes) in both libs and compare final buffers | [x] |
| 19 | `premultiply` | `w=0, h=0` with a non-`NULL` buffer pre-filled with a known pattern — empty shape must leave the buffer untouched (also `ERRORS.md` rows 1–2) | [x] |
| 20 | `premultiply` | `w=0, h>0` and `w>0, h=0` with non-`NULL` buffer — the two one-dimension-empty shapes | [x] |
| 21 | `premultiply` | **negative-x-negative valid-work shape**: `w<0 && h<0` (`(-1,-1)`, `(-2,-3)`, `(-4,-5)`) — bound becomes positive and the loop does real work (`ERRORS.md` row 5); verified against a padded buffer large enough to cover `4*w*h` bytes | [x] |
| 22 | `premultiply` | struct-field aliasing/roundtrip: assert `img->w`, `img->h`, `img->pix` are **unmodified** after the call in both libs (the C never writes the struct) | [x] |
| 23 | `premultiply` | ABI/layout agreement: `size_of`/`offset_of` of `cp_pixel_t` (4/1) and `cp_image_t` (16; `w@0, h@4, pix@8`) match the C compiler's layout, asserted by round-tripping a struct built by the Rust test through **both** `.so`s | [x] |
| 24 | `premultiply` | full-`i32`-range randomized `w,h` with `pix = NULL` (2000 fixed-seed draws), skipping only the combinations whose bound is positive — sweeps the wrapping-arithmetic axis (`ERRORS.md` rows 3–4, 6–10, 15–16) | [x] |

### Feature combinations

`Cargo.toml` has no `[features]`; the only configuration is the default. Every
row above therefore constitutes the complete Phase B matrix for every feature
combination (see Phase D checklist in `SYMBOLS.md`).
