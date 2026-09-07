# CONFIGS.md — Phase A: configuration-surface table

Mechanically derived from the C source. The library has exactly one public entry
point and no runtime options, no global state, no `#ifdef`, and no build flags:

```c
/* c_src/include/lib.h */
unsigned hdr_bitrate(const uint8_t *h);
```

```c
/* c_src/src/lib.c — the single expression the whole library computes */
return 2 * halfrate[ !!((h[1]) & 0x8) ]
                   [ (((h[1]) >> 1) & 3) - 1 ]
                   [ ((h[2]) >> 4) ];
```

So the "configuration" axes are exactly the bit-fields this expression branches
on, plus the shape of the buffer it is handed.

## Axes (grepped out of the source, not guessed)

| axis | source expression | domain |
|------|-------------------|--------|
| A. version/plane bit | `!!((h[1]) & 0x8)` | `{0, 1}` — MPEG-1 vs MPEG-2/2.5 rate plane |
| B. layer field | `(((h[1]) >> 1) & 3) - 1` | raw `{0,1,2,3}` → index `{-1, 0, 1, 2}` (`-1` = reserved layer, out of declared bounds) |
| C. bitrate nibble | `((h[2]) >> 4)` | `{0 .. 15}` (`0` = free format, `15` = "bad", out of declared bounds) |
| D. ignored bits | never read/masked | `h[0]` entirely, `h[1]` bits 0 and 4–7, `h[2]` bits 0–3 |
| E. buffer shape | `h` is only ever dereferenced at `+1` and `+2` | length (3 = exact minimum, 4, 8, huge), alignment (aligned / odd address), offset inside a larger allocation, heap vs stack vs `mmap` |

The A x B x C cross-product is `2 x 4 x 16 = 128` distinct table lookups; the
code distinguishes every one of them, so the table below enumerates all 128 as
8 rows of 16 (grouped by the `(plane, layer)` pair that selects the row), and
then adds the axis-D and axis-E shape rows. Each row is exercised with the full
sweep of the remaining axis plus randomized values (fixed seed) in the ignored
bits, so no row rests on a single hand-picked input.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hdr_bitrate` | plane 0, layer index `-1` (raw layer bits `00`, reserved): all 16 bitrate nibbles, randomized ignored bits | [x] |
| 2 | `hdr_bitrate` | plane 0, layer index `0` (raw `01`, Layer III): all 16 bitrate nibbles, randomized ignored bits | [x] |
| 3 | `hdr_bitrate` | plane 0, layer index `1` (raw `10`, Layer II): all 16 bitrate nibbles, randomized ignored bits | [x] |
| 4 | `hdr_bitrate` | plane 0, layer index `2` (raw `11`, Layer I): all 16 bitrate nibbles, randomized ignored bits | [x] |
| 5 | `hdr_bitrate` | plane 1, layer index `-1` (raw `00`, reserved): all 16 bitrate nibbles, randomized ignored bits | [x] |
| 6 | `hdr_bitrate` | plane 1, layer index `0`: all 16 bitrate nibbles, randomized ignored bits | [x] |
| 7 | `hdr_bitrate` | plane 1, layer index `1`: all 16 bitrate nibbles, randomized ignored bits | [x] |
| 8 | `hdr_bitrate` | plane 1, layer index `2`: all 16 bitrate nibbles, randomized ignored bits | [x] |
| 9 | `hdr_bitrate` | boundary bitrate nibbles only (`0` free-format, `1` lowest, `14` highest valid, `15` bad) crossed with all 8 `(plane, layer)` pairs | [x] |
| 10 | `hdr_bitrate` | axis D: for each of many random `(h[1], h[2])`, compare against the same input with all ignored bits cleared and with all ignored bits set — output must be invariant, and must equal C | [x] |
| 11 | `hdr_bitrate` | axis E: buffer of exactly 3 bytes (minimum the C touches), randomized contents | [x] |
| 12 | `hdr_bitrate` | axis E: header located at a non-zero offset inside a larger buffer (pointer arithmetic path a real caller uses when walking a stream), randomized contents and offsets | [x] |
| 13 | `hdr_bitrate` | axis E: deliberately misaligned (odd-address) header inside a larger allocation, randomized contents | [x] |
| 14 | `hdr_bitrate` | axis E: header at the very end of an `mmap`ed page whose successor page is `PROT_NONE`, so any read past `h[2]` faults; randomized contents | [x] |
| 15 | `hdr_bitrate` | axis E: `h[0]` inside a `PROT_NONE` page, `h[1..2]` in the next readable page, so any read of `h[0]` faults; randomized contents | [x] |
| 16 | `hdr_bitrate` | exhaustive sweep: every one of the `256 x 256 = 65536` `(h[1], h[2])` byte pairs — the complete A x B x C x (low ignored bits) product | [x] |
| 17 | `hdr_bitrate` | repeated / interleaved calls alternating C and Rust with the same buffer, checking there is no hidden state or lazy-init ordering dependence | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is no `add_executable`, and the Rust crate is `crate-type = ["cdylib"]` with no
`[[bin]]`. The project builds **no driver binary**, so the "compare stdout of the
C and Rust binaries" gate is not applicable.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section at all, so the only
configuration is the default (empty) feature set. Verified by running the suite
under `--no-default-features` as well, which is identical here.
