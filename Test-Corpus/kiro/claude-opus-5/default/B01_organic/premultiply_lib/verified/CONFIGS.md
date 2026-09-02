# CONFIGS.md — configuration-surface table (Phase A, gate for Phase B)

## Axes actually present in the C source

Derived mechanically from `c_src/include/lib.h` + `c_src/src/lib.c` and
cross-checked against `objdump -d` of the built `.so`.

### Runtime options / modes / flags — NONE

```
$ grep -cE '#ifdef|#if |switch|enum|flags?|mode|option' c_src/src/lib.c c_src/include/lib.h
0
```

The public API is a single `void premultiply(cp_image_t *)`. There is no
options struct, no flags word, no mode enum, no compile-time `#ifdef` branch,
no byte-order switch, no element-type switch. Consequently the configuration
surface is entirely **input shape**, and there is nothing to cross with it.

### Public entry points (complete set, including the lowest level)

| entry point | level | in table |
|---|---|---|
| `premultiply` | the only one; it *is* the lowest level | yes |

There are no convenience wrappers and no internal helpers (`static` functions):
`premultiply` is simultaneously the highest- and lowest-level entry point.

### Input shape axes the code branches on

| axis | values the C distinguishes | why (source evidence) |
|---|---|---|
| `w` sign/magnitude | `<0`, `0`, `1`, small, 2^28, 2^29, 2^29+1, 2^30, `INT_MAX`, `INT_MIN` | `int stride = w * sizeof(cp_pixel_t)` — `size_t` multiply truncated to `int`; sign and 32-bit wrap change the loop bound |
| `h` sign/magnitude | `<0`, `0`, `1`, small, large, `INT_MAX`, `INT_MIN` | `(int)stride * h` — signed `imul`, wraps |
| pixel count | 0, 1, 2, many (`w*h`) | the loop is the only control flow; 0 vs 1 vs many are the empty / single / general cases |
| alpha byte value | `0`, `1`, mid, `254`, `255` — and all 256 | `a = data[i+3]/255.0f` scales every other channel; `a==0` zeroes RGB, `a==255` is exactly `1.0f` so RGB is preserved bit-exactly, intermediate values are where float rounding + truncation-toward-zero decide the result |
| RGB byte values | `0`, `255`, and all 256 | `(uint8_t)(c/255.0f * a * 255.0f)` — the round-trip is value-dependent, so only full coverage proves it |
| channel role | R/G/B written, A read-only | `data[i+0..2]` are stored; `data[i+3]` is loaded only |
| buffer alignment | 4-byte aligned, and offsets +1/+2/+3 | access is through `uint8_t *`, so misalignment is legal and must behave identically |
| buffer vs geometry | allocation == `w*h*4`, allocation > `w*h*4` | there is no bounds check, so an over-allocated buffer lets the exact walked range be observed |
| geometry aliasing | `w=1,h=N` vs `w=N,h=1` vs `w=a,h=b` with the same product | the loop only ever sees the product `w*4*h`, so equal products must give identical results — a real invariant to assert |

## Configuration rows

Each row is driven with **many randomized inputs** (xorshift64* PRNG, fixed
seed per row, seed printed on failure) unless the row says *exhaustive*.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `premultiply` | **exhaustive value domain**: one 65 536-pixel image containing every `(channel, alpha)` byte pair, with `r == g == b == channel`. Each channel is computed independently by the same expression `(uint8_t)(c/255.0f * (a/255.0f) * 255.0f)`, so all 256×256 pairs is a full proof of the float pipeline for every channel. | [x] |
| 2 | `premultiply` | **exhaustive with distinct channels**: same 65 536 alpha/value sweep but `r`, `g`, `b` set to three *different* bytes per pixel, proving the channels do not interfere and that the write order R→G→B matches | [x] |
| 3 | `premultiply` | `w=2,h=1`; randomized pixels — two pixels, single row | [x] |
| 4 | `premultiply` | `w=1,h=2`; randomized pixels — two pixels, two rows | [x] |
| 5 | `premultiply` | `w=1,h=N` (N random 1..4096), randomized pixels — degenerate single column | [x] |
| 6 | `premultiply` | `w=N,h=1` (N random 1..4096), randomized pixels — degenerate single row | [x] |
| 7 | `premultiply` | `w,h` both random 1..64, randomized pixels — general 2-D case | [x] |
| 8 | `premultiply` | `w,h` random with equal products (e.g. 6x4 vs 24x1 vs 1x24), same pixel data — asserts the product-only invariant holds in both | [x] |
| 9 | `premultiply` | random geometry, **alpha forced to 0** for every pixel — RGB must become 0, alpha must stay 0 | [x] |
| 10 | `premultiply` | random geometry, **alpha forced to 255** for every pixel — RGB must be preserved exactly | [x] |
| 11 | `premultiply` | random geometry, **alpha forced to 1** (smallest non-zero) — worst case for truncation toward zero | [x] |
| 12 | `premultiply` | random geometry, **alpha forced to 128 / 127 / 254** (near-half and near-max) | [x] |
| 13 | `premultiply` | random geometry, RGB forced to `0x00`, alpha random | [x] |
| 14 | `premultiply` | random geometry, RGB forced to `0xFF`, alpha random | [x] |
| 15 | `premultiply` | random geometry, buffer misaligned by +1, +2, +3 bytes from a 4-aligned allocation | [x] |
| 16 | `premultiply` | random geometry, buffer **over-allocated** by 64 pixels; asserts the trailing slack is byte-identical (i.e. both walk exactly `w*4*h` bytes and no further) | [x] |
| 17 | `premultiply` | **idempotence/repeat**: call twice in a row on the same buffer in both libs, compare after each call — catches state or ordering differences | [x] |
| 18 | `premultiply` | `w=3,h=0x4000_0001` — `stride*h` wraps to `+12`; only 3 pixels of a larger buffer may be touched (valid-path side of ERRORS row 13) | [x] |
| 19 | `premultiply` | `w=-2,h=-3` — both dimensions negative, bound wraps *positive* (`+24`), so 6 pixels ARE processed (valid-path side of ERRORS row 6); randomized pixel data | [x] |
| 20 | `premultiply` | `w=-1,h=-1` … `w=-8,h=-8` swept, randomized pixels — negative-times-negative family | [x] |
| 21 | `premultiply` | randomized **fully arbitrary `i32`** `w`/`h` fuzz (both signs, full range), buffer sized from the computed wrapped bound and skipped when that bound exceeds a safety cap; asserts identical behaviour including all the no-op cases | [x] |
| 22 | `premultiply` | randomized geometry, pixel bytes drawn from a **biased** distribution that oversamples `0`, `1`, `127`, `128`, `254`, `255` — concentrates on rounding boundaries | [x] |
| 23 | `premultiply` | `w=1,h=1` — the minimal single-pixel call, randomized (smallest non-empty shape, its own row because it is the boundary of "many") | [x] |

## Check-off

Tests live in `tests/phase_b_valid_paths.rs`, one `rowNN_*` test per row, each
driving both `.so` files through their exported `premultiply` symbol via
`libloading`. Randomized rows use the xorshift64* PRNG in
`tests/support/mod.rs` with a fixed per-row seed, so failures are reproducible.

All 23 rows pass under `dev` and `release`, with and without
`--no-default-features`, and additionally under
`RUSTFLAGS="-C debug-assertions=on -C overflow-checks=on"`.

## Note on axes deliberately NOT in the table

* **Byte order / element type / pixel format.** `cp_pixel_t` is a fixed
  `uint8_t[4]` and the C indexes it as `[0]=r [1]=g [2]=b [3]=a` with no
  alternative layout, so there is no format axis to cross.
* **Alignment of `cp_image_t` itself.** The struct is passed by pointer and read
  with ordinary field access; a misaligned `cp_image_t *` is UB in C in a way
  that is not observably specified, unlike the `pix` buffer which the C
  deliberately reinterprets as `uint8_t *`. Only the `pix` alignment axis is
  therefore meaningful, and it is row 15.
* **Threading / reentrancy.** `premultiply` holds no state: no globals, no
  statics, no allocation. Row 17 covers repeated invocation, which is the
  observable part.
