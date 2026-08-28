# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Mechanical derivation of the axes

The public header exposes exactly one entry point and there is no lower-level or
higher-level API to distinguish — the "lowest-level entry point" *is* the only
entry point:

```c
/* c_src/include/lib.h — the complete public API */
unsigned hdr_bitrate(const uint8_t *h);
```

There is therefore:

* **no runtime option / mode / flag** — no context struct, no setter, no global,
  no `enum` parameter (grep for `if`/`switch`/`#if`/`#ifdef` in the library:
  0 hits);
* **no compile-time configuration** — no `#ifdef` in the library and no
  `[features]` in `Cargo.toml`, so exactly one feature combination exists
  (the default/empty one);
* **no length, count or format parameter** — the argument is a bare pointer.

All configuration is therefore carried *in the input data itself*. The axes are
exactly the bit-fields the C expression branches on, read straight off the
source:

```c
return 2 * halfrate[ !!(h[1] & 0x8) ][ ((h[1] >> 1) & 3) - 1 ][ h[2] >> 4 ];
```

| axis | source of the branch | distinct values |
|------|----------------------|----------------:|
| A. `plane` — MPEG version bit | `!!(h[1] & 0x8)` selects one of the 2 outer planes | 2 |
| B. `layer` — layer bits | `((h[1] >> 1) & 3) - 1` selects the middle index; **4** encodings, of which `00` yields `-1` | 4 |
| C. `rate` — bitrate nibble | `h[2] >> 4` selects the innermost element; **16** encodings, of which `15` overruns the row | 16 |

The C distinguishes every point of the cross product `A x B x C` = **2 x 4 x 16 =
128** combinations (each selects a different flat table offset, in `-15 ..= 90`),
so rows 1–128 below are that full, unpruned cross product. Rows 129–140 add the
input **shape** axes — the memory-layout properties of `*h` that the code's
pointer arithmetic can be sensitive to, plus the bits it must ignore.

Every row is exercised through **both** `.so`s via `libloading`, with the
don't-care bits (`h[1]` bits 0 and 4..7, `h[2]` bits 0..3, `h[0]`, `h[3..]`) and
the buffer contents filled from a **fixed-seed** PRNG over many randomized
iterations per row — never a single hand-picked value. See
`tests/valid_path.rs`.

## Table

### Rows 1–128 — full cross product of the three data axes

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 0 (free-format) — `h[1]&0xE=0x00`, `h[2]&0xF0=0x00`, offset `-15` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 2 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 1 — `h[1]&0xE=0x00`, `h[2]&0xF0=0x10`, offset `-14` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 3 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 2 — `h[1]&0xE=0x00`, `h[2]&0xF0=0x20`, offset `-13` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 4 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 3 — `h[1]&0xE=0x00`, `h[2]&0xF0=0x30`, offset `-12` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 5 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 4 — `h[1]&0xE=0x00`, `h[2]&0xF0=0x40`, offset `-11` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 6 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 5 — `h[1]&0xE=0x00`, `h[2]&0xF0=0x50`, offset `-10` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 7 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 6 — `h[1]&0xE=0x00`, `h[2]&0xF0=0x60`, offset `-9` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 8 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 7 — `h[1]&0xE=0x00`, `h[2]&0xF0=0x70`, offset `-8` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 9 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 8 — `h[1]&0xE=0x00`, `h[2]&0xF0=0x80`, offset `-7` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 10 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 9 — `h[1]&0xE=0x00`, `h[2]&0xF0=0x90`, offset `-6` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 11 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 10 — `h[1]&0xE=0x00`, `h[2]&0xF0=0xa0`, offset `-5` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 12 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 11 — `h[1]&0xE=0x00`, `h[2]&0xF0=0xb0`, offset `-4` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 13 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 12 — `h[1]&0xE=0x00`, `h[2]&0xF0=0xc0`, offset `-3` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 14 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 13 — `h[1]&0xE=0x00`, `h[2]&0xF0=0xd0`, offset `-2` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 15 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 14 — `h[1]&0xE=0x00`, `h[2]&0xF0=0xe0`, offset `-1` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |
| 16 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 00 = RESERVED (inner idx -1), rate 15 (bad) — `h[1]&0xE=0x00`, `h[2]&0xF0=0xf0`, offset `0` (in-object), expect `0` | [x] |
| 17 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 0 (free-format) — `h[1]&0xE=0x02`, `h[2]&0xF0=0x00`, offset `0` (in-object), expect `0` | [x] |
| 18 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 1 — `h[1]&0xE=0x02`, `h[2]&0xF0=0x10`, offset `1` (in-object), expect `8` | [x] |
| 19 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 2 — `h[1]&0xE=0x02`, `h[2]&0xF0=0x20`, offset `2` (in-object), expect `16` | [x] |
| 20 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 3 — `h[1]&0xE=0x02`, `h[2]&0xF0=0x30`, offset `3` (in-object), expect `24` | [x] |
| 21 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 4 — `h[1]&0xE=0x02`, `h[2]&0xF0=0x40`, offset `4` (in-object), expect `32` | [x] |
| 22 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 5 — `h[1]&0xE=0x02`, `h[2]&0xF0=0x50`, offset `5` (in-object), expect `40` | [x] |
| 23 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 6 — `h[1]&0xE=0x02`, `h[2]&0xF0=0x60`, offset `6` (in-object), expect `48` | [x] |
| 24 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 7 — `h[1]&0xE=0x02`, `h[2]&0xF0=0x70`, offset `7` (in-object), expect `56` | [x] |
| 25 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 8 — `h[1]&0xE=0x02`, `h[2]&0xF0=0x80`, offset `8` (in-object), expect `64` | [x] |
| 26 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 9 — `h[1]&0xE=0x02`, `h[2]&0xF0=0x90`, offset `9` (in-object), expect `80` | [x] |
| 27 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 10 — `h[1]&0xE=0x02`, `h[2]&0xF0=0xa0`, offset `10` (in-object), expect `96` | [x] |
| 28 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 11 — `h[1]&0xE=0x02`, `h[2]&0xF0=0xb0`, offset `11` (in-object), expect `112` | [x] |
| 29 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 12 — `h[1]&0xE=0x02`, `h[2]&0xF0=0xc0`, offset `12` (in-object), expect `128` | [x] |
| 30 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 13 — `h[1]&0xE=0x02`, `h[2]&0xF0=0xd0`, offset `13` (in-object), expect `144` | [x] |
| 31 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 14 — `h[1]&0xE=0x02`, `h[2]&0xF0=0xe0`, offset `14` (in-object), expect `160` | [x] |
| 32 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 01 (idx 0), rate 15 (bad) — `h[1]&0xE=0x02`, `h[2]&0xF0=0xf0`, offset `15` (in-object), expect `0` | [x] |
| 33 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 0 (free-format) — `h[1]&0xE=0x04`, `h[2]&0xF0=0x00`, offset `15` (in-object), expect `0` | [x] |
| 34 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 1 — `h[1]&0xE=0x04`, `h[2]&0xF0=0x10`, offset `16` (in-object), expect `8` | [x] |
| 35 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 2 — `h[1]&0xE=0x04`, `h[2]&0xF0=0x20`, offset `17` (in-object), expect `16` | [x] |
| 36 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 3 — `h[1]&0xE=0x04`, `h[2]&0xF0=0x30`, offset `18` (in-object), expect `24` | [x] |
| 37 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 4 — `h[1]&0xE=0x04`, `h[2]&0xF0=0x40`, offset `19` (in-object), expect `32` | [x] |
| 38 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 5 — `h[1]&0xE=0x04`, `h[2]&0xF0=0x50`, offset `20` (in-object), expect `40` | [x] |
| 39 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 6 — `h[1]&0xE=0x04`, `h[2]&0xF0=0x60`, offset `21` (in-object), expect `48` | [x] |
| 40 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 7 — `h[1]&0xE=0x04`, `h[2]&0xF0=0x70`, offset `22` (in-object), expect `56` | [x] |
| 41 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 8 — `h[1]&0xE=0x04`, `h[2]&0xF0=0x80`, offset `23` (in-object), expect `64` | [x] |
| 42 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 9 — `h[1]&0xE=0x04`, `h[2]&0xF0=0x90`, offset `24` (in-object), expect `80` | [x] |
| 43 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 10 — `h[1]&0xE=0x04`, `h[2]&0xF0=0xa0`, offset `25` (in-object), expect `96` | [x] |
| 44 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 11 — `h[1]&0xE=0x04`, `h[2]&0xF0=0xb0`, offset `26` (in-object), expect `112` | [x] |
| 45 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 12 — `h[1]&0xE=0x04`, `h[2]&0xF0=0xc0`, offset `27` (in-object), expect `128` | [x] |
| 46 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 13 — `h[1]&0xE=0x04`, `h[2]&0xF0=0xd0`, offset `28` (in-object), expect `144` | [x] |
| 47 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 14 — `h[1]&0xE=0x04`, `h[2]&0xF0=0xe0`, offset `29` (in-object), expect `160` | [x] |
| 48 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 10 (idx 1), rate 15 (bad) — `h[1]&0xE=0x04`, `h[2]&0xF0=0xf0`, offset `30` (in-object), expect `0` | [x] |
| 49 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 0 (free-format) — `h[1]&0xE=0x06`, `h[2]&0xF0=0x00`, offset `30` (in-object), expect `0` | [x] |
| 50 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 1 — `h[1]&0xE=0x06`, `h[2]&0xF0=0x10`, offset `31` (in-object), expect `32` | [x] |
| 51 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 2 — `h[1]&0xE=0x06`, `h[2]&0xF0=0x20`, offset `32` (in-object), expect `48` | [x] |
| 52 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 3 — `h[1]&0xE=0x06`, `h[2]&0xF0=0x30`, offset `33` (in-object), expect `56` | [x] |
| 53 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 4 — `h[1]&0xE=0x06`, `h[2]&0xF0=0x40`, offset `34` (in-object), expect `64` | [x] |
| 54 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 5 — `h[1]&0xE=0x06`, `h[2]&0xF0=0x50`, offset `35` (in-object), expect `80` | [x] |
| 55 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 6 — `h[1]&0xE=0x06`, `h[2]&0xF0=0x60`, offset `36` (in-object), expect `96` | [x] |
| 56 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 7 — `h[1]&0xE=0x06`, `h[2]&0xF0=0x70`, offset `37` (in-object), expect `112` | [x] |
| 57 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 8 — `h[1]&0xE=0x06`, `h[2]&0xF0=0x80`, offset `38` (in-object), expect `128` | [x] |
| 58 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 9 — `h[1]&0xE=0x06`, `h[2]&0xF0=0x90`, offset `39` (in-object), expect `144` | [x] |
| 59 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 10 — `h[1]&0xE=0x06`, `h[2]&0xF0=0xa0`, offset `40` (in-object), expect `160` | [x] |
| 60 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 11 — `h[1]&0xE=0x06`, `h[2]&0xF0=0xb0`, offset `41` (in-object), expect `176` | [x] |
| 61 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 12 — `h[1]&0xE=0x06`, `h[2]&0xF0=0xc0`, offset `42` (in-object), expect `192` | [x] |
| 62 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 13 — `h[1]&0xE=0x06`, `h[2]&0xF0=0xd0`, offset `43` (in-object), expect `224` | [x] |
| 63 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 14 — `h[1]&0xE=0x06`, `h[2]&0xF0=0xe0`, offset `44` (in-object), expect `256` | [x] |
| 64 | `hdr_bitrate` | plane=0 (MPEG2/2.5), layer bits 11 (idx 2), rate 15 (bad) — `h[1]&0xE=0x06`, `h[2]&0xF0=0xf0`, offset `45` (in-object), expect `0` | [x] |
| 65 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 0 (free-format) — `h[1]&0xE=0x08`, `h[2]&0xF0=0x00`, offset `30` (in-object), expect `0` | [x] |
| 66 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 1 — `h[1]&0xE=0x08`, `h[2]&0xF0=0x10`, offset `31` (in-object), expect `32` | [x] |
| 67 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 2 — `h[1]&0xE=0x08`, `h[2]&0xF0=0x20`, offset `32` (in-object), expect `48` | [x] |
| 68 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 3 — `h[1]&0xE=0x08`, `h[2]&0xF0=0x30`, offset `33` (in-object), expect `56` | [x] |
| 69 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 4 — `h[1]&0xE=0x08`, `h[2]&0xF0=0x40`, offset `34` (in-object), expect `64` | [x] |
| 70 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 5 — `h[1]&0xE=0x08`, `h[2]&0xF0=0x50`, offset `35` (in-object), expect `80` | [x] |
| 71 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 6 — `h[1]&0xE=0x08`, `h[2]&0xF0=0x60`, offset `36` (in-object), expect `96` | [x] |
| 72 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 7 — `h[1]&0xE=0x08`, `h[2]&0xF0=0x70`, offset `37` (in-object), expect `112` | [x] |
| 73 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 8 — `h[1]&0xE=0x08`, `h[2]&0xF0=0x80`, offset `38` (in-object), expect `128` | [x] |
| 74 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 9 — `h[1]&0xE=0x08`, `h[2]&0xF0=0x90`, offset `39` (in-object), expect `144` | [x] |
| 75 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 10 — `h[1]&0xE=0x08`, `h[2]&0xF0=0xa0`, offset `40` (in-object), expect `160` | [x] |
| 76 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 11 — `h[1]&0xE=0x08`, `h[2]&0xF0=0xb0`, offset `41` (in-object), expect `176` | [x] |
| 77 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 12 — `h[1]&0xE=0x08`, `h[2]&0xF0=0xc0`, offset `42` (in-object), expect `192` | [x] |
| 78 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 13 — `h[1]&0xE=0x08`, `h[2]&0xF0=0xd0`, offset `43` (in-object), expect `224` | [x] |
| 79 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 14 — `h[1]&0xE=0x08`, `h[2]&0xF0=0xe0`, offset `44` (in-object), expect `256` | [x] |
| 80 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 00 = RESERVED (inner idx -1), rate 15 (bad) — `h[1]&0xE=0x08`, `h[2]&0xF0=0xf0`, offset `45` (in-object), expect `0` | [x] |
| 81 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 0 (free-format) — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x00`, offset `45` (in-object), expect `0` | [x] |
| 82 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 1 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x10`, offset `46` (in-object), expect `32` | [x] |
| 83 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 2 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x20`, offset `47` (in-object), expect `40` | [x] |
| 84 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 3 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x30`, offset `48` (in-object), expect `48` | [x] |
| 85 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 4 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x40`, offset `49` (in-object), expect `56` | [x] |
| 86 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 5 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x50`, offset `50` (in-object), expect `64` | [x] |
| 87 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 6 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x60`, offset `51` (in-object), expect `80` | [x] |
| 88 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 7 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x70`, offset `52` (in-object), expect `96` | [x] |
| 89 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 8 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x80`, offset `53` (in-object), expect `112` | [x] |
| 90 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 9 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0x90`, offset `54` (in-object), expect `128` | [x] |
| 91 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 10 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0xa0`, offset `55` (in-object), expect `160` | [x] |
| 92 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 11 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0xb0`, offset `56` (in-object), expect `192` | [x] |
| 93 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 12 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0xc0`, offset `57` (in-object), expect `224` | [x] |
| 94 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 13 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0xd0`, offset `58` (in-object), expect `256` | [x] |
| 95 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 14 — `h[1]&0xE=0x0a`, `h[2]&0xF0=0xe0`, offset `59` (in-object), expect `320` | [x] |
| 96 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 01 (idx 0), rate 15 (bad) — `h[1]&0xE=0x0a`, `h[2]&0xF0=0xf0`, offset `60` (in-object), expect `0` | [x] |
| 97 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 0 (free-format) — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x00`, offset `60` (in-object), expect `0` | [x] |
| 98 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 1 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x10`, offset `61` (in-object), expect `32` | [x] |
| 99 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 2 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x20`, offset `62` (in-object), expect `48` | [x] |
| 100 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 3 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x30`, offset `63` (in-object), expect `56` | [x] |
| 101 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 4 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x40`, offset `64` (in-object), expect `64` | [x] |
| 102 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 5 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x50`, offset `65` (in-object), expect `80` | [x] |
| 103 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 6 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x60`, offset `66` (in-object), expect `96` | [x] |
| 104 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 7 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x70`, offset `67` (in-object), expect `112` | [x] |
| 105 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 8 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x80`, offset `68` (in-object), expect `128` | [x] |
| 106 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 9 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0x90`, offset `69` (in-object), expect `160` | [x] |
| 107 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 10 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0xa0`, offset `70` (in-object), expect `192` | [x] |
| 108 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 11 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0xb0`, offset `71` (in-object), expect `224` | [x] |
| 109 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 12 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0xc0`, offset `72` (in-object), expect `256` | [x] |
| 110 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 13 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0xd0`, offset `73` (in-object), expect `320` | [x] |
| 111 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 14 — `h[1]&0xE=0x0c`, `h[2]&0xF0=0xe0`, offset `74` (in-object), expect `384` | [x] |
| 112 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 10 (idx 1), rate 15 (bad) — `h[1]&0xE=0x0c`, `h[2]&0xF0=0xf0`, offset `75` (in-object), expect `0` | [x] |
| 113 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 0 (free-format) — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x00`, offset `75` (in-object), expect `0` | [x] |
| 114 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 1 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x10`, offset `76` (in-object), expect `32` | [x] |
| 115 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 2 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x20`, offset `77` (in-object), expect `64` | [x] |
| 116 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 3 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x30`, offset `78` (in-object), expect `96` | [x] |
| 117 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 4 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x40`, offset `79` (in-object), expect `128` | [x] |
| 118 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 5 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x50`, offset `80` (in-object), expect `160` | [x] |
| 119 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 6 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x60`, offset `81` (in-object), expect `192` | [x] |
| 120 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 7 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x70`, offset `82` (in-object), expect `224` | [x] |
| 121 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 8 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x80`, offset `83` (in-object), expect `256` | [x] |
| 122 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 9 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0x90`, offset `84` (in-object), expect `288` | [x] |
| 123 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 10 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0xa0`, offset `85` (in-object), expect `320` | [x] |
| 124 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 11 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0xb0`, offset `86` (in-object), expect `352` | [x] |
| 125 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 12 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0xc0`, offset `87` (in-object), expect `384` | [x] |
| 126 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 13 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0xd0`, offset `88` (in-object), expect `416` | [x] |
| 127 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 14 — `h[1]&0xE=0x0e`, `h[2]&0xF0=0xe0`, offset `89` (in-object), expect `448` | [x] |
| 128 | `hdr_bitrate` | plane=1 (MPEG1), layer bits 11 (idx 2), rate 15 (bad) — `h[1]&0xE=0x0e`, `h[2]&0xF0=0xf0`, offset `90` (OUTSIDE object -> padding byte 0x00), expect `0` | [x] |

### Rows 129–140 — input-shape / memory-layout axes

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 129 | `hdr_bitrate` | Minimum sufficient buffer: exactly 3 bytes (`h[0..=2]`), heap-allocated. Randomized contents, all 128 data configurations swept inside it. | [x] |
| 130 | `hdr_bitrate` | Realistic 4-byte MP3 frame header (`h[0]=0xFF`, sync bits set in `h[1]`) — the shape a real consumer passes. Randomized over the version/layer/CRC bit combinations. | [x] |
| 131 | `hdr_bitrate` | Large buffer (4096 random bytes), function applied at a random offset — verifies nothing beyond `h[2]` is consulted and no state is carried. | [x] |
| 132 | `hdr_bitrate` | Every pointer alignment `0..=63` within an over-aligned buffer (`uint8_t` loads are alignment-free, so all must agree). | [x] |
| 133 | `hdr_bitrate` | Window placed so `h[2]` is the **last** readable byte of an `mmap`ed region with the next page unmapped. Confirms the read window is exactly `h[1..=2]` in both libraries (a read of `h[3]` would `SIGSEGV`). | [x] |
| 134 | `hdr_bitrate` | Window placed at the **start** of an `mmap`ed region with the preceding page unmapped — confirms neither library reads before `h[0]`. | [x] |
| 135 | `hdr_bitrate` | Ignored bits of `h[1]`: for each of the 8 (plane, layer) settings, sweep all 32 combinations of bits `0` and `4..7`; result must be invariant. | [x] |
| 136 | `hdr_bitrate` | Ignored bits of `h[2]`: for each of the 16 rate nibbles, sweep all 16 low-nibble values; result must be invariant. | [x] |
| 137 | `hdr_bitrate` | `h[0]` swept over all 256 values with `h[1]`, `h[2]` fixed; result must be invariant (`h[0]` is never read). | [x] |
| 138 | `hdr_bitrate` | **Exhaustive**: all 65536 `(h[1], h[2])` pairs with randomized don't-care bytes — the function's complete input domain. Supersedes rows 1–128 and 135–137 as a total check. | [x] |
| 139 | `hdr_bitrate` | Purity / no cross-call state: the two loaded `.so` handles called alternately many times with different inputs (C table is `static const`, Rust's is a `static`). | [x] |
| 140 | `hdr_bitrate` | Return-value width: configurations whose result exceeds `u8` (up to `448`) and results that are exactly `0`; checks the full `unsigned` return marshalled across FFI, incl. upper 32 bits zero when the return is read as `u64`. | [x] |

Rows 1–140 constitute the complete configuration surface. Every row is covered
by a test in `tests/valid_path.rs`; a `[x]` is set only after that row passed
across its randomized inputs against both `.so`s.
