# CONFIGS.md — Configuration surface (valid inputs) of the C library

## Mechanical derivation of the axes

The public header `c_src/include/lib.h` is the whole API surface:

```c
typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;   /* 4 bytes, align 1 */
typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;
void premultiply(cp_image_t *img);
```

**Full set of public entry points: exactly one — `premultiply`.** It is also
the lowest-level entry point; there is no convenience wrapper layer to hide
behind, so every row below drives the real primitive.

There are **no** runtime option/mode/flag parameters, **no** enums, and **no**
`#ifdef`/`#if` in `c_src` (verified by grep, see `ERRORS.md`). Therefore the
configuration axes are the *input shapes* the code branches on, derived from
the one branch in the source (`lib.c:8`) and from its data flow:

| axis | why (source evidence) | values exercised |
|------|-----------------------|------------------|
| **A. `w` (`img->w`)** | `stride = w * sizeof(cp_pixel_t)` (`lib.c:6`), 32-bit wrapping `shl $2` | `0`, `1`, small, large, negative, `INT_MAX`, `INT_MIN`, `0x3FFFFFFF`, `0x40000000`, `0x40000001`, `0x20000000` |
| **B. `h` (`img->h`)** | `(int)stride * h` (`lib.c:8`), 32-bit wrapping `imul` | `0`, `1`, small, large, negative, `INT_MAX`, `INT_MIN`, `2`, `3` |
| **C. sign of `end = (int)stride*h`** | the sole loop guard `i < end` | `end < 0`, `end == 0`, `end > 0` |
| **D. iteration count `end/4`** | `i += 4` step | `0`, `1`, `2`, few, many (10⁵–10⁶) |
| **E. image *shape* for a fixed pixel count** | the loop is flat over `4*w*h` bytes — rows are **not** strided independently, so `1×N`, `N×1`, `M×N` must all collapse to the same contiguous run | `1×1`, `1×N`, `N×1`, `M×N`, `N×M` |
| **F. alpha byte value `data[i+3]`** | `a = data[i+3]/255.0f`, multiplies r,g,b (`lib.c:9,13-15`) | `0`, `1`, `127`, `128`, `254`, `255`, all 256 |
| **G. colour byte values `data[i+0..2]`** | `r/g/b = data[i+k]/255.0f`, then `(uint8_t)(x*255.0f)` truncation (`cvttss2si`) | `0`, `1`, `127`, `128`, `254`, `255`, all 256 |
| **H. alpha channel preservation** | `data[i+3]` is read but **never written** | asserted on every row |
| **I. `pix` pointer alignment** | `data` is a raw `uint8_t*`; no alignment assumption | 4-byte aligned, and offsets `+1/+2/+3` (unaligned) |
| **J. `pix` nullness when no work** | dereferenced only inside the loop | `NULL` with `end<=0` |
| **K. repeated application** | function is not idempotent (truncation loses information) | 1×, 2×, 3× applications |
| **L. touched extent** | writes only bytes `[0, end)`, 3 of every 4 | guard bytes before/after checked byte-exact |

Rows are the pruned cross-product of the axes the C actually distinguishes.
Every row is run against **both** `.so`s and compared byte-for-byte over the
whole pixel buffer **plus** guard regions. Rows marked *randomized* use a
fixed-seed SplitMix64 PRNG (`SEED = 0x5EED_1234_ABCD_F00D`) with many
independent inputs per row (counts noted), so no row depends on one
hand-picked value.

## The configuration-surface table

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `premultiply` | `w=1, h=1` — single pixel; **exhaustive over all 65 536 `(colour, alpha)` byte pairs** applied to r, g and b independently (axes F×G complete) | `cfg01_single_pixel_exhaustive_alpha_colour` | [x] |
| 2  | `premultiply` | `w=1, h=1`, randomized RGBA (4 096 inputs) | `cfg02_single_pixel_random` | [x] |
| 3  | `premultiply` | `w=256, h=256` — one pixel per `(colour,alpha)` combination in a single call: exhaustive core inside a *large* buffer (axes D-many × F × G) | `cfg03_full_byte_cross_product_one_call` | [x] |
| 4  | `premultiply` | `w=N, h=1` single row, `N ∈ {1,2,3,4,5,7,8,15,16,17,31,32,33,63,64,65,255,256,257,1023,1024,1025}`, randomized pixels (16 trials each) | `cfg04_single_row_widths` | [x] |
| 5  | `premultiply` | `w=1, h=N` single column, same `N` set, randomized (16 trials each) | `cfg05_single_column_heights` | [x] |
| 6  | `premultiply` | `w=M, h=N` general 2-D, all 21×21 combinations of the small/medium `M,N` set, randomized | `cfg06_two_dimensional_grid` | [x] |
| 7  | `premultiply` | shape-equivalence: `1×K`, `K×1`, `M×N` with `M*N==K` on identical byte content must all produce identical output (axis E) | `cfg07_shape_equivalence` | [x] |
| 8  | `premultiply` | `w*h` **large**: `1024×1024` (1 048 576 px, 4 MiB) randomized | `cfg08_large_image` | [x] |
| 9  | `premultiply` | alpha pinned per row to each of `{0,1,2,63,64,127,128,129,192,253,254,255}`, colours randomized (`w=64,h=64`, 12 rows) | `cfg09_alpha_pinned_sweep` | [x] |
| 10 | `premultiply` | colours pinned to each of `{0,1,127,128,254,255}`, alpha randomized (`w=64,h=64`, 6 rows) | `cfg10_colour_pinned_sweep` | [x] |
| 11 | `premultiply` | `alpha == 255` (identity alpha) — checks the round-trip `(x/255)*1.0*255` truncation, exhaustive over all 256 colour values | `cfg11_alpha_255_roundtrip` | [x] |
| 12 | `premultiply` | `alpha == 0` — all colour bytes must become `0`, alpha preserved, exhaustive over colours | `cfg12_alpha_zero_zeroes_colours` | [x] |
| 13 | `premultiply` | biased/degenerate byte distributions (all-`0x00`, all-`0xFF`, `0x00/0xFF` only, low-only `0..3`, high-only `252..255`, alternating) on `37×53` | `cfg13_degenerate_distributions` | [x] |
| 14 | `premultiply` | `end == 0` via `w=0` with `h ∈ {0,1,2,7,1000,-1,-1000,INT_MAX,INT_MIN}` — no-op, buffer + guards bitwise identical | `cfg14_w_zero_all_h` | [x] |
| 15 | `premultiply` | `end == 0` via `h=0` with `w ∈ {0,1,2,7,1000,-1,-1000,INT_MAX,INT_MIN}` — no-op | `cfg15_h_zero_all_w` | [x] |
| 16 | `premultiply` | `end < 0`: `w>0, h<0` over `w ∈ {1,2,3,17,1000}` × `h ∈ {-1,-2,-17,-1000}` — no-op | `cfg16_pos_w_neg_h` | [x] |
| 17 | `premultiply` | `end < 0`: `w<0, h>0` over the mirrored set — no-op | `cfg17_neg_w_pos_h` | [x] |
| 18 | `premultiply` | `end > 0` from **both dimensions negative**: `w ∈ {-1,-2,-3,-17,-64}` × `h ∈ {-1,-2,-3,-17,-64}` → `|w*h|` pixels really are processed; randomized pixels | `cfg18_neg_w_neg_h_processes` | [x] |
| 19 | `premultiply` | 32-bit `stride` wrap: `w ∈ {0x3FFFFFFF, 0x40000000, 0x40000001, 0x40000002, 0x7FFFFFFF, -0x40000000, INT_MIN}` × `h ∈ {-2,-1,0,1,2,3}` — full wrap matrix, randomized pixels, byte-exact incl. guards | `cfg19_stride_wrap_matrix` | [x] |
| 20 | `premultiply` | 32-bit `end` wrap: `w ∈ {0x20000000, 0x10000000, 0x08000000, 3, 5}` × `h ∈ {2,3,4,8,16,0x20000000, INT_MAX, INT_MIN}` — full wrap matrix | `cfg20_end_wrap_matrix` | [x] |
| 21 | `premultiply` | `pix` unaligned by `+1`, `+2`, `+3` bytes, `w=29,h=7`, randomized (axis I) | `cfg21_unaligned_pix` | [x] |
| 22 | `premultiply` | `pix == NULL` combined with every `end<=0` configuration (axis J) | `cfg22_null_pix_when_no_work` | [x] |
| 23 | `premultiply` | repeated application 1×/2×/3× on the same buffer (axis K) — composed pipeline, `w=48,h=48`, randomized | `cfg23_repeated_application` | [x] |
| 24 | `premultiply` | touched-extent check: buffer padded with 64 guard bytes before and after `4*w*h`; only bytes `4k`, `4k+1`, `4k+2` for `k < w*h` may change (axis L) | `cfg24_guarded_extent` | [x] |
| 25 | `premultiply` | struct-field ABI: `w`/`h`/`pix` read from offsets `0`/`4`/`8`, `sizeof(cp_image_t)==16`; the same raw 16-byte struct image is handed to both `.so`s, and the struct itself must be unmodified afterwards | `cfg25_struct_abi_and_immutability` | [x] |
| 26 | `premultiply` | randomized *fuzz* over the whole space: `w`,`h` drawn from a mixed distribution (small ints, negatives, powers of two, wrap boundaries, `INT_MIN/MAX`), buffer sized so any in-range access is legal, 20 000 iterations | `cfg26_fuzz_dimensions_and_pixels` | [x] |
| 27 | `premultiply` | **mixed-sign dimensions whose 32-bit wrap makes `end` positive**: `w<0,h>0` and `w>0,h<0` pairs such as `(-0x3FFFFFFF, 1..7)`, `(-0x3FFF_FF00, 1..2)`, `(-0x3FFF_0000, 1/3)`, `(3, -357913941)`, `(1/2, -0x3FFFFFFF)`, `(4, -0x1FFFFFFF)`, `(16, -0x07FFFFFF)` — a negative dimension is **not** always a no-op, so this is a valid-path configuration with real pixel work | `cfg27_mixed_sign_wrapped_positive` | [x] |

## Documented skips

Two combinations in row 20 (`w = 0x08000000` with `h = 2` and `h = 3`) wrap to
`end = 0x40000000` / `0x60000000`, i.e. a 1 GiB / 1.5 GiB pixel buffer per
library. They are skipped because the harness caps a single row's allocation at
`MAX_BYTES = 8 MiB`; the test asserts that the skip set is **exactly** those
two, so any change to the wrap behaviour would be caught. The behaviour they
would exercise — a large positive `end` produced purely by 32-bit wrap — is
covered instead by `w = 0x40000401, h = 1000` (`end = 4_100_000`, ~1 M pixels)
in row 19 and by `w = -0x3FFF_0000, h = 3` (`end = 786_432`) in row 27.

## Feature / build-configuration surface

`Cargo.toml` has **no `[features]` section** and `c_src/CMakeLists.txt` has no
`option()`, no `add_definitions`, and no conditional compilation. The C source
contains no `#if`/`#ifdef`. The complete set of valid feature combinations is
therefore the single empty one:

| # | feature combination | `cargo check` | `cargo test` |
|---|---------------------|---------------|--------------|
| 1 | *(none)* — `--no-default-features` | pass | pass |
| 2 | `--all-features` (equals #1, no features exist) | pass | pass |
| 3 | default (equals #1, no default features declared) | pass | pass |

Additionally, because the Rust translation's faulting behaviour used to depend
on `-C debug-assertions`, every phase is also re-run against a **release**
build of the `.so` (`cargo build --release --lib`, which is where
`[profile.release] panic = "abort"` applies) via the `PREMULT_RUST_SO`
environment variable. See `run_all.sh`.

## Additional rows added while verifying

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| 28 | `premultiply` | **proof by exhaustion for the `float`→`uint8_t` conversion**: all 65 536 `(colour, alpha)` pairs, asserting the intermediate `x*255.0f` is always inside `[0.0, 255.0]` (so the `cvttss2si`-vs-saturating-`as` difference is unreachable) and that the C `.so` matches the exact IEEE-754 f32 chain on every pair | `cfg28_conversion_is_never_out_of_range` | [x] |
