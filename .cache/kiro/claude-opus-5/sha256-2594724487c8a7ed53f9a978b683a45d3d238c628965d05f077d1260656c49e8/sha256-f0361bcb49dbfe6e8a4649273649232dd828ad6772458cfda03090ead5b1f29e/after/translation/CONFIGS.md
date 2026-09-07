# CONFIGS.md — configuration / valid-input surface table (Phase B gate)

## Mechanical derivation of the axes

`c_src/include/lib.h` exposes exactly one entry point, and it is also the
lowest-level one — there is no convenience wrapper and no one-shot API to
prefer over a primitive:

```c
unsigned hdr_bitrate(const uint8_t *h);
```

There is no runtime option, no mode flag, no init/config struct, no global
state, no `#ifdef`, and no byte-order or element-type parameter (confirmed by
the grep in `ERRORS.md`, which finds zero conditionals or preprocessor
branches). The Rust crate likewise declares **no** `[features]`, so the only
feature combination is the default/empty one.

The C therefore branches on nothing *except* the bit-fields it extracts from
the input buffer. Those bit-fields are the configuration axes, read straight out
of the return expression:

```c
return 2 * halfrate[!!((h[1]) & 0x8)][(((h[1]) >> 1) & 3) - 1][((h[2]) >> 4)];
```

| axis | source expression | distinct values the code treats differently |
|------|-------------------|---------------------------------------------|
| `i` — MPEG version / ID bit | `!!(h[1] & 0x8)` | 2: `0` (h[1] bit3 clear), `1` (set) |
| `layer` — layer field | `(h[1] >> 1) & 3` | 4: `0b00` (**reserved** ⇒ row index `-1`), `0b01`, `0b10`, `0b11` |
| `k` — bitrate index nibble | `h[2] >> 4` | 16: `0`(free) .. `14`, and `15` (**bad**/reserved) |
| `h[0]` | not read | 1 (must be irrelevant — asserted, not enumerated) |
| `h[1]` bit0 | not used by any index | 1 (must be irrelevant — asserted, not enumerated) |
| `h[2]` low nibble | shifted out | 1 (must be irrelevant — asserted, not enumerated) |

Pruned cross-product of the axes that actually change the result:
`2 (i) × 4 (layer) × 16 (k)` = **128 rows**, enumerated below. Nothing is
pruned away as "unimportant"; all 128 are listed because the C distinguishes
all 128 (they map to 128 distinct flat table offsets, per `ERRORS.md`).

Each row is exercised with **many randomized inputs** (fixed seed): for a row's
`(i, layer, k)` the free bits — all 8 bits of `h[0]`, `h[1]` bit0, and `h[2]`'s
low nibble — are randomized, so a row passes only if it holds across all the
value combinations that must not matter. In addition, Phase B runs an
**exhaustive** sweep of all 65 536 `(h[1], h[2])` pairs, which is a superset of
every row.

`offset` below is the flat byte index `45*i + 15*(layer-1) + k` that the
compiled C actually loads (see `ERRORS.md`); `OOB<` = before the table,
`OOB>` = past it, `alias` = lands in a different row than declared.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=0 (free) — offset -15 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 2 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=1 — offset -14 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 3 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=2 — offset -13 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 4 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=3 — offset -12 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 5 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=4 — offset -11 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 6 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=5 — offset -10 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 7 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=6 — offset -9 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 8 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=7 — offset -8 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 9 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=8 — offset -7 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 10 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=9 — offset -6 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 11 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=10 — offset -5 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 12 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=11 — offset -4 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 13 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=12 — offset -3 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 14 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=13 — offset -2 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 15 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=14 — offset -1 OOB< before table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 16 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b00 RESERVED, k=15 (bad) — offset 0 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 17 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=0 (free) — offset 0 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 18 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=1 — offset 1 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 19 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=2 — offset 2 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 20 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=3 — offset 3 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 21 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=4 — offset 4 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 22 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=5 — offset 5 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 23 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=6 — offset 6 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 24 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=7 — offset 7 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 25 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=8 — offset 8 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 26 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=9 — offset 9 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 27 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=10 — offset 10 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 28 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=11 — offset 11 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 29 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=12 — offset 12 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 30 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=13 — offset 13 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 31 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=14 — offset 14 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 32 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b01, k=15 (bad) — offset 15 alias (bad nibble reads next row's first byte); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 33 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=0 (free) — offset 15 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 34 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=1 — offset 16 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 35 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=2 — offset 17 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 36 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=3 — offset 18 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 37 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=4 — offset 19 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 38 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=5 — offset 20 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 39 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=6 — offset 21 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 40 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=7 — offset 22 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 41 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=8 — offset 23 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 42 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=9 — offset 24 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 43 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=10 — offset 25 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 44 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=11 — offset 26 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 45 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=12 — offset 27 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 46 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=13 — offset 28 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 47 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=14 — offset 29 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 48 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b10, k=15 (bad) — offset 30 alias (bad nibble reads next row's first byte); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 49 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=0 (free) — offset 30 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 50 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=1 — offset 31 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 51 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=2 — offset 32 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 52 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=3 — offset 33 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 53 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=4 — offset 34 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 54 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=5 — offset 35 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 55 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=6 — offset 36 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 56 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=7 — offset 37 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 57 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=8 — offset 38 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 58 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=9 — offset 39 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 59 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=10 — offset 40 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 60 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=11 — offset 41 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 61 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=12 — offset 42 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 62 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=13 — offset 43 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 63 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=14 — offset 44 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 64 | `hdr_bitrate` | i=0 (h[1]&0x8 clear), layer=0b11, k=15 (bad) — offset 45 alias (bad nibble reads next row's first byte); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 65 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=0 (free) — offset 30 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 66 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=1 — offset 31 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 67 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=2 — offset 32 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 68 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=3 — offset 33 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 69 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=4 — offset 34 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 70 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=5 — offset 35 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 71 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=6 — offset 36 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 72 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=7 — offset 37 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 73 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=8 — offset 38 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 74 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=9 — offset 39 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 75 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=10 — offset 40 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 76 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=11 — offset 41 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 77 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=12 — offset 42 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 78 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=13 — offset 43 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 79 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=14 — offset 44 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 80 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b00 RESERVED, k=15 (bad) — offset 45 alias (reserved layer folds into another row); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 81 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=0 (free) — offset 45 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 82 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=1 — offset 46 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 83 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=2 — offset 47 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 84 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=3 — offset 48 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 85 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=4 — offset 49 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 86 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=5 — offset 50 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 87 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=6 — offset 51 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 88 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=7 — offset 52 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 89 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=8 — offset 53 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 90 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=9 — offset 54 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 91 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=10 — offset 55 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 92 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=11 — offset 56 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 93 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=12 — offset 57 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 94 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=13 — offset 58 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 95 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=14 — offset 59 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 96 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b01, k=15 (bad) — offset 60 alias (bad nibble reads next row's first byte); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 97 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=0 (free) — offset 60 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 98 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=1 — offset 61 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 99 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=2 — offset 62 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 100 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=3 — offset 63 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 101 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=4 — offset 64 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 102 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=5 — offset 65 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 103 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=6 — offset 66 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 104 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=7 — offset 67 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 105 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=8 — offset 68 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 106 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=9 — offset 69 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 107 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=10 — offset 70 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 108 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=11 — offset 71 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 109 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=12 — offset 72 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 110 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=13 — offset 73 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 111 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=14 — offset 74 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 112 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b10, k=15 (bad) — offset 75 alias (bad nibble reads next row's first byte); free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 113 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=0 (free) — offset 75 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 114 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=1 — offset 76 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 115 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=2 — offset 77 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 116 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=3 — offset 78 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 117 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=4 — offset 79 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 118 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=5 — offset 80 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 119 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=6 — offset 81 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 120 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=7 — offset 82 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 121 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=8 — offset 83 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 122 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=9 — offset 84 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 123 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=10 — offset 85 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 124 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=11 — offset 86 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 125 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=12 — offset 87 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 126 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=13 — offset 88 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 127 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=14 — offset 89 in declared bounds; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
| 128 | `hdr_bitrate` | i=1 (h[1]&0x8 set), layer=0b11, k=15 (bad) — offset 90 OOB> past table; free bits h[0]/h[1]bit0/h[2]low-nibble randomized | [x] |
