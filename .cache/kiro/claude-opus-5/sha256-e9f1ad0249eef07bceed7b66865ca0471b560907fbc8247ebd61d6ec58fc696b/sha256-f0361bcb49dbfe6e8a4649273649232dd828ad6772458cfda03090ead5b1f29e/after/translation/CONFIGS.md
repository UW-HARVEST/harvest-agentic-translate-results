# CONFIGS.md — configuration-surface table (valid inputs)

Mechanically derived from the branches `c_src/src/lib.c` actually takes. There is
exactly **one public entry point** (`read_side_info`); the lowest-level routine
(`get_bits`) is `static`, so it is driven directly through the two axes that are
its only inputs besides `n`: the **initial bit alignment** `bs->pos & 7` and the
**limit** `bs->limit` (rows C24-C39). Every row is exercised with many
randomised inputs (fixed seed) via a bit-writer that emits the side-info fields
in exactly the order the C reads them, so each configuration can be hit
deliberately rather than by luck.

## Axes the C branches on

| axis | source line | values |
|------|-------------|--------|
| `M` = MPEG1 flag `hdr[1] & 0x8` | L92, L102, L133, L152 | 0 (MPEG2/2.5) / 1 (MPEG1) |
| `CH` = mono `(hdr[3] & 0xC0) == 0xC0` | L91, L97 | mono / not-mono |
| `gr_count` (derived from `M`x`CH`) | L91-92 | 1, 2, 4 |
| `main_data_begin` read shape | L93-96 | `get_bits(9)` (M=1) / `get_bits(8+gr_count) >> gr_count` (M=0) |
| `scfsi` read shape | L94, L98, L156 | `get_bits(7+gr_count)` (M=1) / left 0 (M=0); `<<= 4` per granule, extra `<<= 4` when mono |
| `W` = window-switching bit | L112 | 0 (long-block branch) / 1 (window-switched branch) |
| `BT` = `block_type` | L113 | 1, 2, 3 (0 = error, see ERRORS.md E3) |
| `MB` = `mixed_block_flag` | L118, L122 | 0 / 1 (only observable when `BT == 2`) |
| table selected | L110, L125-131 | `g_scf_long` / `g_scf_short` / `g_scf_mixed` |
| `sr_idx` | L88-90 | 0..8 (8 = out of range, ERRORS.md E5) |
| `scalefac_compress` width | L108 | 4 bits (M=1) / 9 bits (M=0) |
| `preflag` source | L152 | `get_bits(1)` (M=1) / `scalefac_compress >= 500` (M=0) |
| `n_long_sfb` for mixed | L129 | 8 (M=1) / 6 (M=0) |
| `bs->pos & 7` | `get_bits` L3-5 | 0..7 (masking `255 >> s`, byte-loop trip count) |
| `bs->limit` | `get_bits` L7 | ample / exact / truncated at each field / 0 |
| `part_23_length` sum | L104, L159 | 0 / random / max 4095 per granule |
| `big_values` | L105 | 0 / 1..288 / 289..511 (error) |

`gr_count` is the product of the first two axes:

| M | CH | `gr_count` |
|---|----|-----------|
| 0 | mono | 1 |
| 0 | not-mono | 2 |
| 1 | mono | 2 |
| 1 | not-mono | 4 |

## Rows

Every row: **both** libraries are called through their `.so` exports with the
same buffer pointer and identical pre-filled `bs`/`gr` state; the return value,
the whole `bs_t`, and all 4 granules' 32 bytes (minus the `sfbtab` pointer,
which is compared by dereferencing 23/40 bytes of the pointed-to table) must be
byte-identical. `N` = randomised iterations per row.

### Group 1 — granule-count x long-block path (`W = 0`)

| # | entry point(s) | configuration (options set + input shape) | N | [ ] |
|---|----------------|--------------------------------------------|---|-----|
| C1 | `read_side_info` | M=0, mono, `gr_count`=1, W=0 (long block, `g_scf_long`) | 256 | [x] |
| C2 | `read_side_info` | M=0, not-mono, `gr_count`=2, W=0 | 256 | [x] |
| C3 | `read_side_info` | M=1, mono, `gr_count`=2, W=0 | 256 | [x] |
| C4 | `read_side_info` | M=1, not-mono, `gr_count`=4, W=0 | 256 | [x] |

### Group 2 — window-switched, `block_type = 1` (start block, stays `g_scf_long`)

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C5 | `read_side_info` | M=0, mono, `gr_count`=1, W=1, BT=1 | 256 | [x] |
| C6 | `read_side_info` | M=0, not-mono, `gr_count`=2, W=1, BT=1 | 256 | [x] |
| C7 | `read_side_info` | M=1, mono, `gr_count`=2, W=1, BT=1 | 256 | [x] |
| C8 | `read_side_info` | M=1, not-mono, `gr_count`=4, W=1, BT=1 | 256 | [x] |

### Group 3 — window-switched, `block_type = 3` (stop block, stays `g_scf_long`)

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C9 | `read_side_info` | M=0, mono, `gr_count`=1, W=1, BT=3 | 256 | [x] |
| C10 | `read_side_info` | M=0, not-mono, `gr_count`=2, W=1, BT=3 | 256 | [x] |
| C11 | `read_side_info` | M=1, mono, `gr_count`=2, W=1, BT=3 | 256 | [x] |
| C12 | `read_side_info` | M=1, not-mono, `gr_count`=4, W=1, BT=3 | 256 | [x] |

### Group 4 — window-switched, `BT = 2`, `MB = 0` (short blocks, `g_scf_short`, `scfsi &= 0x0F0F`)

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C13 | `read_side_info` | M=0, mono, `gr_count`=1, W=1, BT=2, MB=0 | 256 | [x] |
| C14 | `read_side_info` | M=0, not-mono, `gr_count`=2, W=1, BT=2, MB=0 | 256 | [x] |
| C15 | `read_side_info` | M=1, mono, `gr_count`=2, W=1, BT=2, MB=0 | 256 | [x] |
| C16 | `read_side_info` | M=1, not-mono, `gr_count`=4, W=1, BT=2, MB=0 | 256 | [x] |

### Group 5 — window-switched, `BT = 2`, `MB = 1` (mixed blocks, `g_scf_mixed`, `n_long_sfb` 6 vs 8)

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C17 | `read_side_info` | M=0, mono, `gr_count`=1, W=1, BT=2, MB=1 (`n_long_sfb`=6) | 256 | [x] |
| C18 | `read_side_info` | M=0, not-mono, `gr_count`=2, W=1, BT=2, MB=1 | 256 | [x] |
| C19 | `read_side_info` | M=1, mono, `gr_count`=2, W=1, BT=2, MB=1 (`n_long_sfb`=8) | 256 | [x] |
| C20 | `read_side_info` | M=1, not-mono, `gr_count`=4, W=1, BT=2, MB=1 | 256 | [x] |

### Group 6 — heterogeneous granules (the composed pipeline: W/BT/MB chosen independently *per granule*)

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C21 | `read_side_info` | M=0, not-mono, `gr_count`=2, W/BT/MB random per granule (mixes long+short+mixed tables and the `scfsi` shift chain in one call) | 2048 | [x] |
| C22 | `read_side_info` | M=1, not-mono, `gr_count`=4, W/BT/MB random per granule | 2048 | [x] |
| C23 | `read_side_info` | M=1, mono, `gr_count`=2, W/BT/MB random per granule (`scfsi <<= 4` twice per granule) | 2048 | [x] |

### Group 7 — `sr_idx` sweep (selects the table row; 8 is one past the end)

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C24 | `read_side_info` | `sr_idx`=0, all three tables (W/BT/MB swept) | 384 | [x] |
| C25 | `read_side_info` | `sr_idx`=1, all three tables | 384 | [x] |
| C26 | `read_side_info` | `sr_idx`=2, all three tables | 384 | [x] |
| C27 | `read_side_info` | `sr_idx`=3, all three tables | 384 | [x] |
| C28 | `read_side_info` | `sr_idx`=4, all three tables | 384 | [x] |
| C29 | `read_side_info` | `sr_idx`=5, all three tables | 384 | [x] |
| C30 | `read_side_info` | `sr_idx`=6, all three tables | 384 | [x] |
| C31 | `read_side_info` | `sr_idx`=7, all three tables | 384 | [x] |
| C32 | `read_side_info` | `sr_idx`=8 (out of range, ERRORS.md E5) — all fields except the `sfbtab` target compared | 384 | [x] |

### Group 8 — initial bit alignment `bs->pos & 7` (drives `get_bits`' `255 >> s` mask and byte-loop trip count)

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C33 | `read_side_info` -> `get_bits` | `pos & 7` = 0, random path | 256 | [x] |
| C34 | `read_side_info` -> `get_bits` | `pos & 7` = 1, random path | 256 | [x] |
| C35 | `read_side_info` -> `get_bits` | `pos & 7` = 2, random path | 256 | [x] |
| C36 | `read_side_info` -> `get_bits` | `pos & 7` = 3, random path | 256 | [x] |
| C37 | `read_side_info` -> `get_bits` | `pos & 7` = 4, random path | 256 | [x] |
| C38 | `read_side_info` -> `get_bits` | `pos & 7` = 5, random path | 256 | [x] |
| C39 | `read_side_info` -> `get_bits` | `pos & 7` = 6, random path | 256 | [x] |
| C40 | `read_side_info` -> `get_bits` | `pos & 7` = 7, random path | 256 | [x] |
| C41 | `read_side_info` -> `get_bits` | large byte-aligned `pos` (deep into the buffer, `pos >> 3` large) | 256 | [x] |

### Group 9 — `bs->limit` shapes (drives the `get_bits` sentinel and the final bounds check)

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C42 | `read_side_info` | limit ample (whole buffer), all paths | 512 | [x] |
| C43 | `read_side_info` | limit **exactly** at the last bit of side info (boundary: last read succeeds) | 512 | [x] |
| C44 | `read_side_info` | limit exactly one bit short (last read trips the sentinel) | 512 | [x] |
| C45 | `read_side_info` | limit swept over **every** bit position from 0 to end-of-side-info (truncation at each field) | 1 per pos, all paths | [x] |

### Group 10 — `preflag` / `scalefac_compress`

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C46 | `read_side_info` | M=0 (9-bit `scalefac_compress`), value < 500 -> `preflag` 0 | 256 | [x] |
| C47 | `read_side_info` | M=0, value >= 500 (i.e. 500..511) -> `preflag` 1 | 256 | [x] |
| C48 | `read_side_info` | M=0, value **exactly 499 / 500** (threshold boundary) | 256 | [x] |
| C49 | `read_side_info` | M=1 (4-bit `scalefac_compress`), `preflag` read from the bitstream (0 and 1) | 256 | [x] |

### Group 11 — value extremes of the parsed fields

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C50 | `read_side_info` | all fields all-zero bits (`part_23_length`=0, `big_values`=0, `main_data_begin`=0) | 32 | [x] |
| C51 | `read_side_info` | all fields all-one bits (max `part_23_length`=4095, `main_data_begin` max, `table_select`=31, `region_count` max, `subblock_gain`=7) | 32 | [x] |
| C52 | `read_side_info` | `big_values` at the legal maximum 288 (E2 boundary, must **not** reject) | 256 | [x] |
| C53 | `read_side_info` | `main_data_begin` at max (511 for M=1, 255/511 for M=0) with large `part_23_sum` (exercises the final comparison both ways) | 512 | [x] |

### Group 12 — unconstrained fuzz over the whole input space

| # | entry point(s) | configuration | N | [ ] |
|---|----------------|---------------|---|-----|
| C54 | `read_side_info` | fully random `hdr[0..4]`, random buffer, random `pos`, random `limit` — no constraints (covers every `M`x`CH`x`sr_idx`x`W`x`BT`x`MB` combination reachable from raw bytes, incl. reserved bit patterns) | 200 000 | [x] |
| C55 | `read_side_info` | exhaustive `hdr[1]`x`hdr[2]`x`hdr[3]` (all 2^24), fixed buffer/pos/limit | 16 777 216 | [x] |

## Feature combinations

`translation/Cargo.toml` declares no `[features]` and no optional dependencies,
so there is a single build configuration; `--no-default-features` is byte-identical
to the default build. There is no `[[bin]]` target, so there is no binary-stdout
comparison to make. See `SYMBOLS.md`.

## What each row actually compares

Per iteration, both libraries are called through `dlopen`/`dlsym` with the
**same** `buf` pointer and identical pre-filled state, and the following must be
byte-identical:

- the `int` return value;
- the whole `bs_t` after the call (`buf`, `pos`, `limit`) — this pins the
  `get_bits` side effect on `pos`, including when the limit sentinel fires;
- bytes 8..32 of **all four** `L3_gr_info_t` slots (every field except the
  `sfbtab` pointer), pre-filled with `0xA5` so fields the C leaves untouched —
  notably `region_count[2]` on the window-switched path, which the C never
  writes — are compared meaningfully and the *number of granules written* is
  pinned;
- whether `sfbtab` was written at all;
- the 23 (long) or 40 (short/mixed) bytes `sfbtab` points at, so the three
  `g_scf_*` tables are compared through the FFI boundary for every `sr_idx`
  row. Skipped only for `sr_idx == 8`, where the C forms an out-of-range address
  (ERRORS.md E5) and the bytes past each library's own table are unrelated.

## Harness sensitivity

`scripts/mutation_check.sh` injects 47 single-edit mutations into `src/lib.rs`,
rebuilds the cdylib and re-runs the full suite for each. **47/47 are killed.**
The mutations cover: individual bytes of all three `g_scf_*` tables, every
`get_bits` internal (shift init `n+s`, loop step, final `>> -shl`, the
`255 >> s` mask, the `pos` advance, the pointer base), every field bit-width,
every threshold/mask/sentinel constant, the `sr_idx` computation and table
strides, the granule pointer advance, and the MPEG1/mono header tests. This is
the evidence that the rows above are not passing vacuously.

## Build profiles covered

The full suite passes against both the release cdylib (the shipped artifact) and
the debug cdylib (`RUST_SO=target/debug/libread_side_info_lib.so`). Because debug
builds enable overflow checks, the debug run additionally proves that no
arithmetic in the translation can panic on the C's wrapping/overflowing paths.
