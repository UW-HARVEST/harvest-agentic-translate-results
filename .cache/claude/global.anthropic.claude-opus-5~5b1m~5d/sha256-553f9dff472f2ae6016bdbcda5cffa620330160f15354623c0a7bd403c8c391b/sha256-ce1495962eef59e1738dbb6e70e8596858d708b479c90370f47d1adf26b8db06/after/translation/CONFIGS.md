# CONFIGS.md — Configuration surface table (Phase B)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C actually branches on

The library has **no runtime options, modes, or flags** — there is no init
struct, no context object, no setter, and no `#ifdef` in the source. The public
API is a single function, and it is also the *lowest-level* entry point (there
is no convenience wrapper layered over a lower-level core, so "exercise the
low-level entry points, not just the wrappers" is satisfied by definition —
`flip_horizontal` *is* the low level).

Everything the code distinguishes therefore comes from **input shape**, i.e. the
three fields of `cp_image_t` plus the buffer contents:

| axis | values the code special-cases | where |
|------|-------------------------------|-------|
| `h` (row count) parity | **even** → every row paired; **odd** → the middle row `h/2` is never touched | `flips = h / 2` |
| `h` magnitude | `0`, `1` → `flips == 0`, no work; `2`,`3` → one flip; `≥4` → multiple flips | outer guard `i < flips` |
| `h` sign | negative → `flips <= 0` → no work | outer guard |
| `w` (row width) magnitude | `0` → inner loop never runs; `1` → single pixel per row (a==b never, but degenerate stride); `≥2` → real per-pixel walk | inner guard `j < w` |
| `w` sign | negative → inner loop never runs, and row pointers go out of bounds (computed only) | inner guard |
| row stride interaction | rows are addressed as `pix + w*i`, so the buffer must be exactly `w*h` — any `(w,h)` pair with the same product exercises a *different* pairing | `pix + w*i`, `pix + w*(h-i-1)` |
| `pix` | non-null valid `w*h` buffer; **null tolerated iff no work is due** | dereferenced only inside inner loop |
| pixel byte values | all 4 channels are copied verbatim; `0x00`, `0xFF`, and random bytes must all round-trip (catches partial-channel / alpha-dropping bugs) | `cp_pixel_t t = *a; *a = *b; *b = t;` |
| struct write-back | the function must **not** modify `img->w`, `img->h`, `img->pix` | (it never assigns them) |

## Table — one row per meaningful combination

Every row is driven through **both** `.so` exports with **many randomized pixel
buffers** (deterministic SplitMix64, fixed seed `0x5EED_1234_ABCD_EF01`), plus
the all-`0x00` and all-`0xFF` extremes, and compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `flip_horizontal` | `w=0, h=0`, `pix` = null — fully degenerate | [x] |
| 2 | `flip_horizontal` | `w=0, h=0`, `pix` = valid 0-len (dangling-but-aligned) | [x] |
| 3 | `flip_horizontal` | `w=8, h=0` — zero rows, non-empty width | [x] |
| 4 | `flip_horizontal` | `w=1, h=1` — smallest non-empty, odd, no flip | [x] |
| 5 | `flip_horizontal` | `w=8, h=1` — single row, odd, no flip | [x] |
| 6 | `flip_horizontal` | `w=0, h=4` — outer loop spins, inner never runs, `pix` null | [x] |
| 7 | `flip_horizontal` | `w=0, h=5` — same, odd `h` | [x] |
| 8 | `flip_horizontal` | `w=1, h=2` — one swap of one pixel (minimal real work) | [x] |
| 9 | `flip_horizontal` | `w=1, h=3` — odd, middle row must be preserved | [x] |
| 10 | `flip_horizontal` | `w=2, h=2` — smallest multi-pixel row | [x] |
| 11 | `flip_horizontal` | `w=8, h=2` — one flip, wide row | [x] |
| 12 | `flip_horizontal` | `w=8, h=3` — odd `h`, wide row, middle preserved | [x] |
| 13 | `flip_horizontal` | `w=8, h=4` — multiple flips, even | [x] |
| 14 | `flip_horizontal` | `w=3, h=5` — multiple flips, odd, non-power-of-2 width | [x] |
| 15 | `flip_horizontal` | `w=1, h=64` — degenerate width, many flips, even | [x] |
| 16 | `flip_horizontal` | `w=1, h=65` — degenerate width, many flips, odd | [x] |
| 17 | `flip_horizontal` | `w=64, h=1` — wide single row (transpose of #15's shape) | [x] |
| 18 | `flip_horizontal` | `w=37, h=64` — large even, non-power-of-2 width | [x] |
| 19 | `flip_horizontal` | `w=37, h=65` — large odd, non-power-of-2 width | [x] |
| 20 | `flip_horizontal` | `w=256, h=2` — row wider than a cache line / vectorizable | [x] |
| 21 | `flip_horizontal` | `w=2, h=256` — many flips, narrow | [x] |
| 22 | `flip_horizontal` | same `w*h` product, different factorisations (`w×h` = 1×24, 2×12, 3×8, 4×6, 6×4, 8×3, 12×2, 24×1) over the *same* 24-pixel buffer — isolates the `pix + w*i` row-addressing | [x] |
| 23 | `flip_horizontal` | pixel values = all `0x00` (across a representative shape set) | [x] |
| 24 | `flip_horizontal` | pixel values = all `0xFF` (across a representative shape set) | [x] |
| 25 | `flip_horizontal` | pixel values = per-channel distinguishable pattern (`r=idx, g=!idx, b=0xA5, a=idx*7`) — catches channel swap / alpha loss | [x] |
| 26 | `flip_horizontal` | **idempotence/involution**: applying the op twice restores the original, checked on both libs (`h` even and odd) | [x] |
| 27 | `flip_horizontal` | **struct not mutated**: `img->w`, `img->h`, `img->pix` identical after the call, all shapes | [x] |
| 28 | `flip_horizontal` | **no out-of-bounds writes**: buffer padded with guard pixels before/after; guards must be untouched by both libs | [x] |
| 29 | `flip_horizontal` | randomized sweep: 400 random `(w,h)` in `1..=24` with random pixel bytes | [x] |
| 30 | `flip_horizontal` | randomized sweep incl. degenerate/negative dims: 400 random `(w,h)` in `-4..=12` | [x] |

Rows 1–28 are enumerated shapes/data patterns; rows 29–30 are the property-style
randomized sweeps that cover the cross-product beyond the hand-listed points.
