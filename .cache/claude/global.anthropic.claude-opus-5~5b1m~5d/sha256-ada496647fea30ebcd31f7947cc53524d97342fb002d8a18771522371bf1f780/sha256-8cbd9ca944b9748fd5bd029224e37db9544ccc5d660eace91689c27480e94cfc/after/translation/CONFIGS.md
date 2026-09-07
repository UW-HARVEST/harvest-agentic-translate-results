# CONFIGS.md — configuration-surface table (valid inputs)

## Axes the C actually branches on

Derived from every `if` / `?:` / `while` in `c_src/src/lib.c`. There are no
runtime options struct, no `#ifdef`, and no global state — every "option" is a
bit of the 4-byte `hdr` or a bit of the bitstream, so those are the axes.

### From `hdr` (the caller-supplied "options")

| axis | source | values | what it toggles |
|------|--------|--------|-----------------|
| A. `mpeg1` | `hdr[1] & 0x8` | 0, 1 | `gr_count *= 2`; `main_data_begin` = 9 bits vs `(8+gr_count)` bits `>> gr_count`; whether a `(7+gr_count)`-bit `scfsi` is read up front; `scalefac_compress` width 4 vs 9 bits; `preflag` = 1 read bit vs `scalefac_compress >= 500`; mixed-block `n_long_sfb` 8 vs 6 |
| B. `mono` | `(hdr[3] & 0xC0) == 0xC0` | 0, 1 | base `gr_count` 1 vs 2; extra `scfsi <<= 4` at the top of every granule iteration |
| C. `ext_bit` | `(hdr[1] >> 4) & 1` | 0, 1 | contributes `*3` to `sr_idx` |
| D. `sr_bits` | `(hdr[2] >> 2) & 3` | 0,1,2,3 | `sr_idx` base (3 == reserved) |
| — derived | `sr_idx = D + (A + C)*3`, then `-1` if non-zero | 0..8 | which scalefactor-band row `sfbtab` points at; 8 is out of range (see ERRORS E8) |
| — derived | `gr_count` | 1, 2, 4 | number of granules written / loop iterations |

Reachable `gr_count`: `mono && !mpeg1` → 1; (`!mono && !mpeg1`) or
(`mono && mpeg1`) → 2; `!mono && mpeg1` → 4.
Reachable `sr_idx`: `!A,!C` → 0,1,2 (D=0 gives 0); `A xor C` → 2,3,4,5;
`A,C` → 5,6,7,8.

### From the bitstream (per granule — input *shape*)

| axis | source | values | what it toggles |
|------|--------|--------|-----------------|
| E. `window_switching` | 1 bit | 0, 1 | long path (15-bit `tables`, 4+3-bit `region_count`, `region_count[2]=255`, `subblock_gain` untouched) vs short path (10-bit `tables<<5`, `region_count = {7,255,untouched}`, three 3-bit `subblock_gain`) |
| F. `block_type` | 2 bits (only if E=1) | 0 (→ error), 1, 2, 3 | only 2 selects short/mixed tables and the `scfsi &= 0x0F0F` mask |
| G. `mixed_block_flag` | 1 bit (only if E=1) | 0, 1 | only relevant when F==2: `g_scf_short` + `{0,39}` sfb vs `g_scf_mixed` + `{6 or 8, 30}` sfb; `region_count[0]` 8 vs 7 |
| H. `big_values` | 9 bits | 0..288 valid, 289..511 → error | ERRORS E3 |
| I. `scalefac_compress` | 4 or 9 bits | 0..15 / 0..511 | `>= 500` → `preflag=1` (non-MPEG1 only) |
| J. `part_23_length` | 12 bits × gr_count | 0..4095 each | summed into the final overflow check (ERRORS E5) |

### Buffer / limit shapes

| axis | values | why |
|------|--------|-----|
| K. `bs->pos` on entry | 0, 1..7 (unaligned), byte-aligned >0, large | `s = pos & 7` masks the first byte; `p = buf + (pos>>3)` |
| L. `bs->limit` | ≫ needed, exactly the bits needed, one bit short, 0, negative, `INT_MAX` | drives ERRORS E1/E2/E9 |
| M. `main_data_begin` | 0, small, max (511 / 1023>>gr_count) | scales the final check's right-hand side |

### Build-time configuration (Cargo features)

| axis | values | what it toggles |
|------|--------|-----------------|
| N. `c_layout_o2` | off (default), on | Which `.rodata` array order the Rust blob reproduces: `-O0` (long, pad, short, mixed) vs `-O2` (mixed, short, long). Observable only via the out-of-range `sr_idx == 8` row. Each setting is tested against the C build that produces that order, over BOTH cargo profiles — 4 feature combos x 2 profiles = 8 configurations, every row below re-run in each (`run_all.sh`). |

### Entry points

The C library has exactly ONE public entry point, `read_side_info` (see
SYMBOLS.md). `get_bits` is `static` and therefore *not* separately callable
across the FFI boundary; it is exercised indirectly but exhaustively, because
`read_side_info` calls it with every `n` the code uses — `n ∈ {1, 2, 3, 4, 8,
9, 10, 11, 12, 15}` — at every one of the 8 possible `s = pos & 7` phases. Tests
`c30_c38_all_eight_start_phases` / `c32_unaligned_mid_buffer` cover that
10 × 8 cross-product by driving `read_side_info` from every start phase with
both `mpeg1` settings (so the `n=4` vs `n=9` and `n=9` vs `n=11` variants both
occur).

There is no binary/driver target in `c_src/CMakeLists.txt` (`add_library(...
SHARED ...)` only) and none in `translation/Cargo.toml` (`crate-type =
["cdylib"]`, no `[[bin]]`), so the "compare stdout of the two binaries" step
does not apply.

## Configuration rows

Every row is exercised with **many randomized bitstreams** (`ChaCha`-free
xorshift64* PRNG, fixed seed `0x5DEE_CE66_D000_1234`, 96..512 iterations per
row and up to 200 000 for the fuzz row) so that
`part_23_length`, `big_values`, `global_gain`, `scalefac_compress`,
`table_select`, `region_count`, `subblock_gain`, `scfsi` and the residual bits
all take many values per row. Each row asserts: return value, `bs->pos`,
`bs->limit`, all 32 struct bytes of every granule, and the dereferenced
`sfbtab` row contents.

### Core cross-product: (A) mpeg1 × (B) mono × (E/F/G) block shape

| #   | entry point | configuration (options set + input shape) | [x] |
|-----|-------------|--------------------------------------------|-----|
| C1  | `read_side_info` | mpeg1=0, mono=1 (gr_count=1), window_switching=0 (long block) | [x] |
| C2  | `read_side_info` | mpeg1=0, mono=1, ws=1, block_type=1 | [x] |
| C3  | `read_side_info` | mpeg1=0, mono=1, ws=1, block_type=2, mixed=0 (short) | [x] |
| C4  | `read_side_info` | mpeg1=0, mono=1, ws=1, block_type=2, mixed=1 (mixed, n_long_sfb=6) | [x] |
| C5  | `read_side_info` | mpeg1=0, mono=1, ws=1, block_type=3 | [x] |
| C6  | `read_side_info` | mpeg1=0, mono=0 (gr_count=2), ws=0 both granules | [x] |
| C7  | `read_side_info` | mpeg1=0, mono=0, ws=1 both granules, block_type=1 | [x] |
| C8  | `read_side_info` | mpeg1=0, mono=0, ws=1 both, block_type=2, mixed=0 | [x] |
| C9  | `read_side_info` | mpeg1=0, mono=0, ws=1 both, block_type=2, mixed=1 | [x] |
| C10 | `read_side_info` | mpeg1=0, mono=0, ws=1 both, block_type=3 | [x] |
| C11 | `read_side_info` | mpeg1=1, mono=1 (gr_count=2), ws=0 both | [x] |
| C12 | `read_side_info` | mpeg1=1, mono=1, ws=1 both, block_type=1 | [x] |
| C13 | `read_side_info` | mpeg1=1, mono=1, ws=1 both, block_type=2, mixed=0 | [x] |
| C14 | `read_side_info` | mpeg1=1, mono=1, ws=1 both, block_type=2, mixed=1 (n_long_sfb=8) | [x] |
| C15 | `read_side_info` | mpeg1=1, mono=1, ws=1 both, block_type=3 | [x] |
| C16 | `read_side_info` | mpeg1=1, mono=0 (gr_count=4), ws=0 all four | [x] |
| C17 | `read_side_info` | mpeg1=1, mono=0, ws=1 all four, block_type=1 | [x] |
| C18 | `read_side_info` | mpeg1=1, mono=0, ws=1 all four, block_type=2, mixed=0 | [x] |
| C19 | `read_side_info` | mpeg1=1, mono=0, ws=1 all four, block_type=2, mixed=1 | [x] |
| C20 | `read_side_info` | mpeg1=1, mono=0, ws=1 all four, block_type=3 | [x] |

### Mixed per-granule shapes (the interaction rows — different granules take different branches)

| #   | entry point | configuration | [x] |
|-----|-------------|---------------|-----|
| C21 | `read_side_info` | mpeg1=0, mono=0: granule 0 long, granule 1 short(bt=2,mixed=0) | [x] |
| C22 | `read_side_info` | mpeg1=0, mono=0: granule 0 short(bt=2,mixed=1), granule 1 long | [x] |
| C23 | `read_side_info` | mpeg1=1, mono=0 (4 granules): per-granule ws/block_type/mixed all randomized independently | [x] |
| C24 | `read_side_info` | mpeg1=1, mono=1 (2 granules): granule 0 bt=2/mixed=0, granule 1 bt=1 (checks `scfsi &= 0x0F0F` then `<<=4` interaction, ERRORS E13) | [x] |
| C25 | `read_side_info` | mpeg1=1, mono=0: only granule 2 has block_type=2 (scfsi mask applied mid-loop) | [x] |

### `sr_idx` axis (C, D) — all 9 reachable values, crossed with the 3 table kinds

| #   | entry point | configuration | [x] |
|-----|-------------|---------------|-----|
| C26 | `read_side_info` | every reachable `(mpeg1, ext_bit, sr_bits)` triple → `sr_idx` 0..8, long block (`g_scf_long[sr_idx]`, 23 bytes compared) | [x] |
| C27 | `read_side_info` | every reachable triple → `sr_idx` 0..8, bt=2/mixed=0 (`g_scf_short[sr_idx]`, 40 bytes compared) | [x] |
| C28 | `read_side_info` | every reachable triple → `sr_idx` 0..8, bt=2/mixed=1 (`g_scf_mixed[sr_idx]`, 40 bytes compared; `sr_idx==8` contents skipped per ERRORS N5) | [x] |

### Channel-mode axis (B) — all 4 raw values of `hdr[3] >> 6`

| #   | entry point | configuration | [x] |
|-----|-------------|---------------|-----|
| C29 | `read_side_info` | `hdr[3]>>6` ∈ {0 stereo, 1 joint, 2 dual, 3 mono} × mpeg1 ∈ {0,1}, randomized bitstream and low 6 bits of `hdr[3]` | [x] |

### Bit-position / limit shapes (K, L, M)

| #   | entry point | configuration | [x] |
|-----|-------------|---------------|-----|
| C30 | `read_side_info` | start `bs->pos` = 0..7 (all 8 phases), generous limit | [x] |
| C31 | `read_side_info` | start `bs->pos` byte-aligned mid-buffer (8, 64, 800) | [x] |
| C32 | `read_side_info` | start `bs->pos` unaligned mid-buffer (11, 67, 803) | [x] |
| C33 | `read_side_info` | `bs->limit` = exactly the number of bits the side info consumes | [x] |
| C34 | `read_side_info` | `bs->limit` = one bit less than consumed (truncation mid-field, all 4 gr_counts) | [x] |
| C35 | `read_side_info` | `bs->limit` swept over every value from 0 to full side-info length (every possible truncation point) | [x] |
| C36 | `read_side_info` | `bs->limit` = `i32::MAX`, `main_data_begin` maximal | [x] |
| C37 | `read_side_info` | `main_data_begin` = 0 / mid / max, with `part_23_length` chosen to land exactly on, one below and one above the E5 boundary | [x] |

### `get_bits` coverage (indirect but complete)

| #   | entry point | configuration | [x] |
|-----|-------------|---------------|-----|
| C38 | `read_side_info` | all `n ∈ {1,2,3,4,8,9,10,11,12,15}` × all 8 phases `s = pos & 7` (driven by C30/C32 with both `mpeg1` values so `n=4`/`n=9` and `n=9`/`n=11` both occur) | [x] |

### Exhaustive header sweep

| #   | entry point | configuration | [x] |
|-----|-------------|---------------|-----|
| C39 | `read_side_info` | all 2^24 combinations of `hdr[1] × hdr[2] × hdr[3]` are too many, so: all 256 × 256 combos of `hdr[1] × hdr[3]` with `hdr[2]` and the bitstream randomized, plus all 256 values of `hdr[2]` — every raw byte value crosses the FFI | [x] |
| C40 | `read_side_info` | pure fuzz: fully random `hdr[0..4]`, random 512-byte buffer, random `pos`/`limit` in valid ranges, 200 000 iterations | [x] |

## Execution

```
$ ./run_all.sh          # all 40 rows x 4 feature combos x 2 profiles
...
   test result: ok. 40 passed; 0 failed   (phase_b_configs)
   test result: ok. 20 passed; 0 failed   (phase_c_errors)
 ALL PHASES PASSED
```

Every row above is checked off because it passed in **all 8** configurations,
not just the default one.
