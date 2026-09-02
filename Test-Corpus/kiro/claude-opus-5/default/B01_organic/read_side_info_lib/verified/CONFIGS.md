# CONFIGS.md — configuration surface table (valid inputs)

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Public entry points

| entry point | linkage | reachable from outside? |
|-------------|---------|-------------------------|
| `read_side_info(bs_t*, L3_gr_info_t*, const uint8_t*)` | external (`nm -D`: `T`) | yes — the only exported symbol |
| `get_bits(bs_t*, int)` | `static` (internal) | **no** — cannot be called across the FFI boundary. It is the lowest-level unit and is covered indirectly: every row drives it through the full set of widths `n ∈ {1,2,3,4,8,9,10,11,12,15}` and every start alignment `bs->pos & 7 ∈ 0..7` (rows `*.9`), which is the complete cross-product of its two behavioural axes (`shl = n + s` loop-iteration count, and the `pos+n > limit` early return covered in `ERRORS.md` E1/E2). |

## Axes the C branches on

| axis | source site | values |
|------|-------------|--------|
| `EXT` = `hdr[1] & 0x8` | `L91` (`gr_count *= 2`, `main_data_begin`/`scfsi` framing), `L110` (`scalefac_compress` width 4 vs 9), `L128` (`n_long_sfb` 8 vs 6), `L152` (`preflag` read vs derived) | `0`, `8` |
| `MONO` = `(hdr[3] & 0xC0) == 0xC0` | `L90` (`gr_count` 1 vs 2), `L99` (extra `scfsi <<= 4` per granule) | false, true |
| `gr_count` | derived: `(MONO?1:2) * (EXT?2:1)` | 1, 2, 4 |
| `sr_idx` | `L87–L89`, then `sr_idx -= (sr_idx != 0)` | `EXT==0` ⇒ `0..5`; `EXT==8` ⇒ `2..8` (see reachability below) |
| per-granule block config | `L114` `window_switching`, `L115` `block_type`, `L118` `mixed_block_flag`, `L122` `block_type==2`, `L124` `!mixed_block_flag` | `L` (ws=0), `S1` (ws=1,bt=1), `S2M0` (ws=1,bt=2,mixed=0), `S2M1` (ws=1,bt=2,mixed=1), `S3` (ws=1,bt=3) |
| selected `sfbtab` / `n_long_sfb` / `n_short_sfb` | `L111–L113`, `L125–L132` | `g_scf_long`/22/0 (L,S1,S3); `g_scf_short`/0/39 (S2M0); `g_scf_mixed`/(8 if EXT else 6)/30 (S2M1) |
| `scfsi` masking | `L123` `scfsi &= 0x0F0F` iff any granule has `block_type == 2` | on / off, and interacts with the `MONO` extra shift |
| `preflag` source | `L152` | `EXT==8`: 1 bit from stream (0/1); `EXT==0`: `scalefac_compress >= 500` (9-bit field ⇒ both outcomes reachable) |
| start alignment | `get_bits` `s = bs->pos & 7` | `0..7` |
| `main_data_begin` framing | `L93` (9 bits, EXT) vs `L96` (`get_bits(8+gr_count) >> gr_count`, non-EXT) | 2 shapes |
| reservoir size | `L159` final check | ample / exactly-equal / (overrun ⇒ `ERRORS.md` E5) |

`sr_idx` reachability (`b3 = (hdr[1]>>3)&1` is the same bit as `EXT`,
`b4 = (hdr[1]>>4)&1`, `base = (hdr[2]>>2)&3`):

| b3 | b4 | `base` | `sum` | `sr_idx` |
|----|----|--------|-------|----------|
| 0 | 0 | 0..3 | 0..3 | 0,0,1,2 |
| 0 | 1 | 0..3 | 3..6 | 2,3,4,5 |
| 1 | 0 | 0..3 | 3..6 | 2,3,4,5 |
| 1 | 1 | 0..3 | 6..9 | 5,6,7,**8** ← out of range, see `ERRORS.md` U1 |

## Row table

Each row is run with **many randomised inputs** (fixed-seed PCG32, seed
`0x5EED_0001`): every unconstrained bitfield (`part_23_length`, `big_values ≤ 288`,
`global_gain`, `scalefac_compress`, `tables`, `region_count`, `subblock_gain`,
`scfsi`, `main_data_begin`, the trailing buffer bytes and `bs->limit`) is
redrawn per iteration. Both `.so`s are called with byte-identical inputs and the
full 8-granule output array + `bs` + return value are compared byte-for-byte.

`blocks` column: `all X` = every granule uses config `X`; `seq [...]` = the
listed configs cycled across granules; `rand` = each granule drawn at random.

### Group A — `EXT=0`, stereo (`gr_count = 2`, 9-bit `scalefac_compress`, derived `preflag`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| A1 | `read_side_info` | EXT=0, stereo, sr_idx=0, blocks=`all L`, align=0, 256 iters | [x] |
| A2 | `read_side_info` | EXT=0, stereo, sr_idx=0, blocks=`all S1`, align=0, 256 iters | [x] |
| A3 | `read_side_info` | EXT=0, stereo, sr_idx=0, blocks=`all S2M0` (→ `g_scf_short`), align=0, 256 iters | [x] |
| A4 | `read_side_info` | EXT=0, stereo, sr_idx=0, blocks=`all S2M1` (→ `g_scf_mixed`, n_long=6), align=0, 256 iters | [x] |
| A5 | `read_side_info` | EXT=0, stereo, sr_idx=0, blocks=`all S3`, align=0, 256 iters | [x] |
| A6 | `read_side_info` | EXT=0, stereo, sr_idx=0, blocks=`seq [L, S2M0]` (mixed pipeline in one call) | [x] |
| A7 | `read_side_info` | EXT=0, stereo, sr_idx=0, blocks=`seq [S2M1, L]` (scfsi mask then not) | [x] |
| A8 | `read_side_info` | EXT=0, stereo, **sr_idx sweep 0..5**, blocks=`rand`, align=0 | [x] |
| A9 | `read_side_info` | EXT=0, stereo, sr_idx=`rand`, blocks=`rand`, **align sweep 0..7** | [x] |
| A10 | `read_side_info` | EXT=0, stereo, sr_idx=`rand`, blocks=`all L`, `scalefac_compress ≥ 500` ⇒ preflag=1 | [x] |
| A11 | `read_side_info` | EXT=0, stereo, sr_idx=`rand`, blocks=`all S2M1`, `scalefac_compress < 500` ⇒ preflag=0 | [x] |
| A12 | `read_side_info` | EXT=0, stereo, everything random (hdr bits, sr_idx, blocks, align, limit), 1024 iters | [x] |

### Group B — `EXT=0`, mono (`gr_count = 1`, extra `scfsi <<= 4` per granule)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| B1 | `read_side_info` | EXT=0, mono, sr_idx=0, blocks=`all L` | [x] |
| B2 | `read_side_info` | EXT=0, mono, sr_idx=0, blocks=`all S1` | [x] |
| B3 | `read_side_info` | EXT=0, mono, sr_idx=0, blocks=`all S2M0` | [x] |
| B4 | `read_side_info` | EXT=0, mono, sr_idx=0, blocks=`all S2M1` | [x] |
| B5 | `read_side_info` | EXT=0, mono, sr_idx=0, blocks=`all S3` | [x] |
| B6 | `read_side_info` | EXT=0, mono, sr_idx=0, blocks=`seq [L, S2M0]` | [x] |
| B7 | `read_side_info` | EXT=0, mono, sr_idx=0, blocks=`seq [S2M1, L]` | [x] |
| B8 | `read_side_info` | EXT=0, mono, **sr_idx sweep 0..5**, blocks=`rand` | [x] |
| B9 | `read_side_info` | EXT=0, mono, sr_idx=`rand`, blocks=`rand`, **align sweep 0..7** | [x] |
| B10 | `read_side_info` | EXT=0, mono, `scalefac_compress ≥ 500` ⇒ preflag=1, blocks=`all L` | [x] |
| B11 | `read_side_info` | EXT=0, mono, `scalefac_compress < 500` ⇒ preflag=0, blocks=`all S2M1` | [x] |
| B12 | `read_side_info` | EXT=0, mono, everything random, 1024 iters | [x] |

### Group C — `EXT=8`, stereo (`gr_count = 4`, 4-bit `scalefac_compress`, `preflag` read from stream, `n_long_sfb=8` for mixed)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `read_side_info` | EXT=8, stereo, sr_idx=2, blocks=`all L` | [x] |
| C2 | `read_side_info` | EXT=8, stereo, sr_idx=2, blocks=`all S1` | [x] |
| C3 | `read_side_info` | EXT=8, stereo, sr_idx=2, blocks=`all S2M0` | [x] |
| C4 | `read_side_info` | EXT=8, stereo, sr_idx=2, blocks=`all S2M1` (n_long=8 here, 6 in group A/B) | [x] |
| C5 | `read_side_info` | EXT=8, stereo, sr_idx=2, blocks=`all S3` | [x] |
| C6 | `read_side_info` | EXT=8, stereo, sr_idx=7, blocks=`seq [L, S2M0, S2M1, S3]` (all four granules different) | [x] |
| C7 | `read_side_info` | EXT=8, stereo, sr_idx=7, blocks=`seq [S2M1, L]` | [x] |
| C8 | `read_side_info` | EXT=8, stereo, **sr_idx sweep 2..8** (incl. out-of-range 8), blocks=`rand` | [x] |
| C9 | `read_side_info` | EXT=8, stereo, sr_idx=`rand`, blocks=`rand`, **align sweep 0..7** | [x] |
| C10 | `read_side_info` | EXT=8, stereo, `preflag` stream bit forced 1, blocks=`all L` | [x] |
| C11 | `read_side_info` | EXT=8, stereo, `preflag` stream bit forced 0, blocks=`all S2M1` | [x] |
| C12 | `read_side_info` | EXT=8, stereo, everything random, 1024 iters | [x] |

### Group D — `EXT=8`, mono (`gr_count = 2`, extra `scfsi <<= 4`, 4-bit `scalefac_compress`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| D1 | `read_side_info` | EXT=8, mono, sr_idx=2, blocks=`all L` | [x] |
| D2 | `read_side_info` | EXT=8, mono, sr_idx=2, blocks=`all S1` | [x] |
| D3 | `read_side_info` | EXT=8, mono, sr_idx=2, blocks=`all S2M0` | [x] |
| D4 | `read_side_info` | EXT=8, mono, sr_idx=2, blocks=`all S2M1` | [x] |
| D5 | `read_side_info` | EXT=8, mono, sr_idx=2, blocks=`all S3` | [x] |
| D6 | `read_side_info` | EXT=8, mono, sr_idx=8 (out-of-range), blocks=`seq [L, S2M0]` | [x] |
| D7 | `read_side_info` | EXT=8, mono, sr_idx=8 (out-of-range), blocks=`seq [S2M1, S1]` | [x] |
| D8 | `read_side_info` | EXT=8, mono, **sr_idx sweep 2..8**, blocks=`rand` | [x] |
| D9 | `read_side_info` | EXT=8, mono, sr_idx=`rand`, blocks=`rand`, **align sweep 0..7** | [x] |
| D10 | `read_side_info` | EXT=8, mono, `preflag` stream bit forced 1, blocks=`all S2M0` | [x] |
| D11 | `read_side_info` | EXT=8, mono, `preflag` stream bit forced 0, blocks=`all L` | [x] |
| D12 | `read_side_info` | EXT=8, mono, everything random, 1024 iters | [x] |

### Group X — cross-cutting shapes

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| X1 | `read_side_info` | `sr_idx == 8` in every group where reachable; `gr->sfbtab` must land exactly one row past the table (`base + 8*rowsize`) for long/short/mixed | [x] |
| X2 | `read_side_info` | reservoir exactly exhausted: `part_23_sum + pos == limit + main_data_begin*8` (accept boundary, `ERRORS.md` E7) | [x] |
| X3 | `read_side_info` | large `main_data_begin` (up to 511 on the EXT path, up to 63 on the non-EXT path) so the final check passes with a tiny `limit` | [x] |
| X4 | `read_side_info` | `hdr[0]` varied over 0..255 with everything else fixed — must not change the result (`ERRORS.md` U4) | [x] |
| X5 | `read_side_info` | full-space fuzz: `hdr[1..4]` uniformly random (all 2^24 combinations of the bits the code reads), random 256-byte buffer, random `pos` 0..63, random `limit`, 20 000 iterations | [x] |
| X6 | `read_side_info` | `bs->pos` non-zero and byte-misaligned combined with `limit` small enough that `get_bits` starts returning 0 part-way through granule 2 (partial-decode shape) | [x] |
| X7 | `read_side_info` | `scfsi` field all-ones / all-zeros / alternating, across `MONO` and `EXT` (drives `scfsi <<= 4`, `&= 0x0F0F`, `(scfsi>>12)&15` interaction) | [x] |
| X8 | `read_side_info` | granule-count boundary: `gr_count = 1, 2, 4` each verified to write exactly that many granules and leave granules `gr_count..8` untouched (`ERRORS.md` U2) | [x] |
| X9 | `read_side_info` | exhaustive table-literal check: all 3 tables × all 8 in-range rows, forced deliberately, whole row compared byte-for-byte (pins down all 824 table constants incl. the implicit zero-fill of the short C initialiser lists in `g_scf_mixed`) | [x] |

**Total: 57 rows.**

## Phase B status

All rows above are checked off; `cargo test --release --test phase_b_configs`
reports **57 passed, 0 failed** (one test function per row, named after the row
id — `a1_…`, `b7_…`, `x9_…`).

Row-to-test mapping is 1:1 by the row id prefix in the test name. Iteration
counts per row: 256 randomised inputs for the fixed-configuration rows, 128 per
`sr_idx` for the sweep rows, 8 per (byte-offset, bit-offset) pair for the
alignment rows, 1024 for the per-group random rows, 20 000 for `X5`, and full
enumeration for `X4` (256 `hdr[0]` values) / `X6` (16 starts × 64 limits × 4
groups) / `X9` (all table rows).

## Verification robustness

* The harness compares the return value, `bs.pos`, `bs.limit`, and all
  8 × 32 output bytes. Because `gr->sfbtab` necessarily holds a different address
  in each shared object, it is compared as a normalised `(table, byte offset)`
  pair — the table is derived from the observable `n_long_sfb` / `n_short_sfb`
  output, the offset is measured against per-library base addresses recovered by
  calibration, and the offset is additionally required to equal
  `sr_idx * row_size`. For in-range `sr_idx` the pointed-to row itself is
  compared byte-for-byte.
* `mutation_check.sh` is the negative control: it injects 30 deliberate bugs into
  `src/lib.rs` (bounds, widths, shifts, masks, strides, table literals, the
  `>` vs `>=` in the reservoir check, …), rebuilds, and requires the suite to
  fail for each. All 30 are caught, so the green run above is meaningful.
* The suite was also run against the C compiled at `-O0/-O1/-O2/-O3/-Os`
  (via `HARVEST_C_SO=...`) — identical results at every level — and under the
  Rust debug profile, where arithmetic overflow checks are enabled and no
  overflow panic occurs.
