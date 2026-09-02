# CONFIGS.md — configuration / valid-input surface table

## How the axes were derived (from the C source, not from assumptions)

Public API (`c_src/include/lib.h`), in full:

```c
typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;   /* 4 bytes */
typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;
void flip_horizontal(cp_image_t *img);
```

* **Public entry points: exactly one** — `flip_horizontal`. There are no
  convenience wrappers and no lower-level helpers (no `static` functions, no
  additional translation units — see `SYMBOLS.md`), so "exercise the lowest
  level entry point too" collapses to this single function. Every row below
  drives it directly through the `.so` export.
* **Runtime options / modes / flags: none.** `grep -n "if\|switch\|#ifdef\|#if \|enum\|#define"`
  over `src/lib.c` + `include/lib.h` returns 0 hits. The only behavioral inputs
  are the three struct fields `w`, `h`, `pix` (plus the pixel bytes themselves).

Axes the C code actually branches on (`src/lib.c`):

| axis | why it matters (the exact C expression) |
|------|----------------------------------------|
| `h` magnitude | `flips = h / 2` controls the outer trip count |
| `h` parity | odd `h` leaves the middle row (`i == h/2`) untouched; even `h` swaps every row |
| `w` magnitude | `j < w` controls the inner trip count |
| `w` vs `h` ratio | `w*i` / `w*(h-i-1)` row strides — wide-flat vs tall-thin shapes stress different offset arithmetic |
| pixel byte values | the swap is byte-copying; value-dependent bugs (channel mix-up, `a` channel dropped) only show with distinct per-channel data |
| `pix` base offset / alignment | `pix` is caller-supplied; the code assumes nothing, so an interior (non-allocation-start, 4-but-not-8-byte-aligned) `pix` must work and must not touch bytes outside `[pix, pix + w*h)` |
| repeated invocation | row swapping is an involution; a real consumer round-trips, which detects off-by-one in `h-i-1` that a single call can mask |

Negative / null / boundary-integer configurations are the *rejection* mirror of
this table and live in `ERRORS.md` (rows 1–19) so they are not duplicated here.

Every row is driven with **many randomized inputs** (SplitMix64, fixed seed
`0x5EED_1234_ABCD_0001`, `REPS` iterations per row) — never a single hand-picked
value — and asserts the **entire** backing buffer (payload + guard bands) is
byte-identical between the C `.so` and the Rust `.so`.

## Configuration-surface table

| #  | entry point(s) | configuration (options set + input shape) | test | ✅ |
|----|----------------|-------------------------------------------|------|----|
| 1  | `flip_horizontal` | `w == 0, h == 0` — fully empty image, non-null `pix` | `cfg_01_empty_0x0` | [x] |
| 2  | `flip_horizontal` | `h == 0`, `w` random `1..=64` — zero rows, non-zero width | `cfg_02_h0_random_w` | [x] |
| 3  | `flip_horizontal` | `h == 1`, `w` random `1..=64` — single row (odd, `flips == 0`) | `cfg_03_h1_random_w` | [x] |
| 4  | `flip_horizontal` | `w == 0`, `h` random `2..=32` — zero-width rows, outer loop *does* iterate | `cfg_04_w0_random_h` | [x] |
| 5  | `flip_horizontal` | `w == 1, h == 2` — minimal real swap (one pixel per row) | `cfg_05_min_swap_1x2` | [x] |
| 6  | `flip_horizontal` | `h == 2`, `w` random `1..=64` — single swap, many widths | `cfg_06_h2_random_w` | [x] |
| 7  | `flip_horizontal` | `h == 3`, `w` random `1..=64` — smallest odd height with work; middle row must stay put | `cfg_07_h3_random_w` | [x] |
| 8  | `flip_horizontal` | `h` random **even** `4..=32`, `w` random `1..=64` | `cfg_08_even_h_random_w` | [x] |
| 9  | `flip_horizontal` | `h` random **odd** `5..=33`, `w` random `1..=64` | `cfg_09_odd_h_random_w` | [x] |
| 10 | `flip_horizontal` | `w == 1`, `h` random `2..=256` — tall-thin (one column) | `cfg_10_tall_thin` | [x] |
| 11 | `flip_horizontal` | `h == 2`, `w` random `512..=4096` — wide-flat rows, large stride | `cfg_11_wide_flat` | [x] |
| 12 | `flip_horizontal` | `w > h` random (`w` `17..=200`, `h` `2..=16`) — landscape | `cfg_12_landscape` | [x] |
| 13 | `flip_horizontal` | `h > w` random (`w` `2..=16`, `h` `17..=200`) — portrait | `cfg_13_portrait` | [x] |
| 14 | `flip_horizontal` | `w == h` random `2..=48` — square | `cfg_14_square` | [x] |
| 15 | `flip_horizontal` | large image, `w*h` up to ~120k pixels, randomized shape | `cfg_15_large_image` | [x] |
| 16 | `flip_horizontal` | double application (`flip` then `flip`) on random `w`/`h` — involution round-trip | `cfg_16_double_application_involution` | [x] |
| 17 | `flip_horizontal` | `pix` points into the **interior** of a larger allocation with 64-byte poison guard bands on both sides, random shape — verifies no byte outside `[pix, pix+w*h)` is touched | `cfg_17_interior_pix_with_guards` | [x] |
| 18 | `flip_horizontal` | `pix` deliberately 4-byte-but-not-8/16-byte aligned, random shape | `cfg_18_unaligned_pix` | [x] |
| 19 | `flip_horizontal` | degenerate pixel payloads: all `0x00`, all `0xFF`, per-channel constant, row-index-stamped, alternating — crossed with random `w`/`h` | `cfg_19_degenerate_payloads` | [x] |
| 20 | `flip_horizontal` | full randomized cross-product sweep `w in 0..=9` × `h in 0..=9` (100 shape combinations incl. all parities and zeros), random payload each | `cfg_20_small_shape_cross_product` | [x] |
| 21 | `flip_horizontal` | same `cp_image_t` struct reused across consecutive calls with `w`/`h` mutated between calls (stateless-ness / no hidden static state) | `cfg_21_struct_reuse_mutating_dims` | [x] |

All 21 rows pass under all four configurations (`dev`/`release` × default /
`--no-default-features`) — see `SYMBOLS.md` for how to reproduce.

## What this table intentionally does not have

No row varies an option, mode, flag, format, byte order or element type,
because **the C exposes none**: the whole public surface is one `void` function
taking one struct pointer, and `grep` finds no `if`, `switch`, `#ifdef`,
`#define` or `enum` anywhere in `src/lib.c` / `include/lib.h`. The axes above
are the complete set of things the C actually branches on. Recorded explicitly
so the short list reads as a verified finding rather than an unexamined
assumption.
