# CONFIGS.md — Phase B configuration-surface table

## How this table was derived

### Runtime options / modes / flags: NONE

Mechanical grep of the entire C source for branch and configuration constructs
(`if`, `switch`, `#ifdef`, `#if`, `enum`, comparison operators) returns **no
matches** (see `ERRORS.md`). `include/lib.h` declares no flags, no mode enum, no
context/handle struct, and no setter functions. `CMakeLists.txt` defines no
`target_compile_definitions`, so there is no compile-time configuration axis
either. **The option axis is empty**: there is exactly one behavior mode.

### Entry points: one, and it is already the lowest level

The public API is the single function `void premultiply(cp_image_t *img)`. There
is no convenience wrapper vs. low-level split to worry about — this *is* the
low-level entry point, and every row below calls it directly through the `.so`
export.

### Input-shape axes the C actually branches on

Everything the C distinguishes falls out of three expressions:

| axis | expression in the C | why it matters |
|------|---------------------|----------------|
| **trip count** | `stride = wrap32(w << 2)`; `end = wrap32(stride * h)`; iterations `= end/4` if `end > 0` else `0` | `w` and `h` only ever reach the code through this wrapping product, so the shape axis is really "which `end` does `(w,h)` produce": zero, small, exact, or wrap-truncated |
| **row-agnostic walk** | `data` is walked as a flat byte run over `[0, end)`, never per row | any `(w,h)` with the same product must give byte-identical results — a real equivalence to test |
| **value dependence** | `v/255.0f`, `* a`, `* 255.0f`, `cvttss2si` | the float round-trip and truncate-toward-zero make the output depend on the *value* of each byte, so `a = 0`, `a = 255`, and mid-range alphas are genuinely different paths |

`stride` is always a multiple of 4 and `wrap32` preserves that, so `end` is
always a multiple of 4: **there is no partial-tail code path**. Alignment of
`cp_pixel_t` is 1 (four `uint8_t`s), so a byte-misaligned `pix` is a legal input
shape and gets its own rows.

Every row is exercised through both `.so` exports with **many seeded random
inputs** (fixed seed, reproducible), not one hand-picked value, and every
comparison is over the full padded buffer so out-of-span writes are caught too.

## Table

Options column is `—` on every row because the option axis is provably empty.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `premultiply` | — ; `w=1,h=1`, **exhaustive** sweep of all 256 channel values × all 256 alphas (65536 pixels) — full proof of the float round-trip and `cvttss2si` truncation | [x] |
| 2 | `premultiply` | — ; `w=1,h=1`, exhaustive `(r,g,b)` = `(v, 255-v, v/2)` × all 256 alphas — proves the three channels are handled independently and identically | [x] |
| 3 | `premultiply` | — ; `w=1,h=1`, seeded random pixels ×20000 (single-pixel minimum shape) | [x] |
| 4 | `premultiply` | — ; `w=256,h=1`, ramp data `pix[i] = (i,i,i,i)` (one row, many pixels) | [x] |
| 5 | `premultiply` | — ; `w=256,h=256` (65536 px), seeded random — large multi-row | [x] |
| 6 | `premultiply` | — ; `w=2,h=3` seeded random ×500 (small even dims) | [x] |
| 7 | `premultiply` | — ; `w=3,h=5` seeded random ×500 (odd, non-power-of-two dims) | [x] |
| 8 | `premultiply` | — ; `w=7,h=1` seeded random ×500 (single row, odd width) | [x] |
| 9 | `premultiply` | — ; `w=1,h=7` seeded random ×500 (single column) | [x] |
| 10 | `premultiply` | — ; `w=64,h=64` seeded random ×20 | [x] |
| 11 | `premultiply` | — ; `w=1000,h=1` seeded random (wide row) | [x] |
| 12 | `premultiply` | — ; **byte-misaligned `pix`**: offsets 1, 2, 3, 5, 7 into the arena, `w=5,h=3`, seeded random | [x] |
| 13 | `premultiply` | — ; **shape equivalence**: `(6,4)`, `(24,1)`, `(8,3)`, `(4,6)`, `(1,24)`, `(2,12)`, `(3,8)`, `(12,2)` on identical 24-px data — all must produce the same bytes in both libs | [x] |
| 14 | `premultiply` | — ; all-zero buffer, `w=16,h=4` (`a=0`, `rgb=0`) | [x] |
| 15 | `premultiply` | — ; all-`0xFF` buffer, `w=16,h=4` (`a=255`, `rgb=255` — max round-trip) | [x] |
| 16 | `premultiply` | — ; `a=0` on every pixel, random `rgb`, `w=32,h=2` (rgb must all become 0) | [x] |
| 17 | `premultiply` | — ; `a=255` on every pixel, random `rgb`, `w=32,h=2` (`v/255*1.0*255` round-trip + truncation) | [x] |
| 18 | `premultiply` | — ; `a=1` (smallest non-zero alpha), random `rgb`, `w=32,h=2` | [x] |
| 19 | `premultiply` | — ; `a=254` (one below max), random `rgb`, `w=32,h=2` | [x] |
| 20 | `premultiply` | — ; `a=128` (mid), random `rgb`, `w=32,h=2` | [x] |
| 21 | `premultiply` | — ; `rgb=0`, random `a`, `w=32,h=2` (rgb stays 0, alpha untouched) | [x] |
| 22 | `premultiply` | — ; **repeated application**: call `premultiply` twice on the same buffer, `w=16,h=16` seeded random — compares the composed pipeline, not one call | [x] |
| 23 | `premultiply` | — ; **negative-`w` and negative-`h` quirk on the valid path**: `w=-2,h=-3` → `end=24` → 6 px processed, seeded random data (also `(-1,-1)`, `(-4,-4)`, `(-3,-5)`) | [x] |
| 24 | `premultiply` | — ; `w=INT_MAX,h=-1` → `end=4` → exactly 1 px processed, seeded random data | [x] |
| 25 | `premultiply` | — ; `w=65536,h=16385` → `end` wraps to `262144` → exactly 65536 px processed out of a nominal 2^30, seeded random 256 KiB arena | [x] |
| 26 | `premultiply` | — ; `w=0x7FFFFFFE,h=0x7FFFFFFE` → `end=16` → exactly 4 px processed, seeded random data | [x] |
| 27 | `premultiply` | — ; **guard-region integrity**: 64-byte poison padding before and after the live span, `w=5,h=3` random — asserts neither lib writes outside `[0,end)` and both leave the guards identical | [x] |
| 28 | `premultiply` | — ; **randomized shape fuzz**: 20000 seeded iterations with `w,h` drawn from `[-9,9]` ∪ boundary values and fully random pixel data — cross-product sweep of the trip-count and value axes together | [x] |

## Binary executable

`c_src/CMakeLists.txt` contains only `add_library(... SHARED src/lib.c)` — there
is no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. **The project builds no driver
binary**, so the "compare C and Rust stdout" clause of Phase B does not apply.
This is asserted mechanically by `tests/feature_matrix.rs`.
