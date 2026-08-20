# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Public entry points

`c_src/include/lib.h` declares exactly **one** function:

```c
int read_side_info(bs_t *bs, L3_gr_info_t *gr, const uint8_t *hdr);
```

`get_bits` is `static` (file-local, not exported by either `.so` — see
`SYMBOLS.md`), so the lowest-level *reachable* entry point is
`read_side_info` itself. It is nevertheless driven at the bit level: every test
synthesises the raw side-info bitstream field by field with a `BitWriter` that
is the exact dual of `get_bits` (MSB-first, `bs->pos`-relative), and sets
`bs->pos` / `bs->limit` / `bs->buf` by hand, so all of `get_bits`' internal
paths (byte-straddling, 1..15-bit widths, all 8 start alignments, negative and
overflowing positions, the truncation early-out) are reached directly rather
than through a convenience wrapper. The `L3_gr_info_t` output array is
pre-filled with a sentinel pattern so that *unwritten* fields are observable.

## Axes the C actually branches on

| axis | source evidence | values |
|------|-----------------|--------|
| `A` = `hdr[1] & 0x8` (MPEG1 flag) | `lib.c:91, 110, 131, 152` | 0 (MPEG2/2.5 layout) / 8 (MPEG1 layout) |
| `M` = `(hdr[3] & 0xC0) == 0xC0` (mono) | `lib.c:90, 99` | mono / not-mono |
| ⇒ `gr_count` | `lib.c:90-92, 158` | 1 (A=0,mono), 2 (A=0,¬mono **or** A=8,mono), 4 (A=8,¬mono) |
| header-field width for `main_data_begin` | `lib.c:93, 96` | A=8: 9 bits; A=0: `8+gr_count` bits then `>> gr_count` (9→1 or 10→2) |
| `scfsi` field width | `lib.c:94` | A=8 only: `7+gr_count` = 9 or 11 bits; A=0: no scfsi read at all (stays 0) |
| extra `scfsi <<= 4` per granule | `lib.c:99-101` | taken only when mono |
| `sr_idx` = `((hdr[2]>>2)&3) + (((hdr[1]>>3)&1)+((hdr[1]>>4)&1))*3`, then `-= (sr_idx!=0)` | `lib.c:87-89` | 0..5 when A=0; 2..8 when A=8 (bit 3 *is* `A`). 8 is out of range for the `[8][…]` tables |
| `scalefac_compress` width | `lib.c:110` | A=8: 4 bits; A=0: 9 bits |
| window-switching flag `W` | `lib.c:114` | 0 (normal, 15-bit `tables` + `region_count[0..1]` + `region_count[2]=255`) / 1 (10-bit `tables<<5` + 3×`subblock_gain`, `region_count[2]` **left untouched**) |
| `block_type` (W=1) | `lib.c:115-134` | 0 ⇒ error, 1, 3 ⇒ long table + `region_count = {7,255,·}`, 2 ⇒ short/mixed |
| `mixed_block_flag` (block_type=2) | `lib.c:124-133` | 0 ⇒ `g_scf_short[sr_idx]`, `region_count[0]=8`, `n_long_sfb=0`, `n_short_sfb=39`; 1 ⇒ `g_scf_mixed[sr_idx]`, `n_long_sfb = A?8:6`, `n_short_sfb=30` |
| `scfsi &= 0x0F0F` | `lib.c:123` | only when block_type == 2 (per granule ⇒ affects *later* granules' `gr->scfsi`) |
| `preflag` source | `lib.c:151-152` | A=8: 1 read bit; A=0: `scalefac_compress >= 500` |
| `bs->pos & 7` start alignment | `lib.c:4, 6, 9` | 0..7 (+ negative `pos` ⇒ arithmetic `>>`) |
| `bs->limit` | `lib.c:7` | ample / exactly at the last needed bit / cut mid-stream (see `ERRORS.md`) |
| `part_23_sum` vs `limit + mdb*8` | `lib.c:159` | pass / exact boundary / fail |

Per-granule axes (`W`, `block_type`, `mixed_block_flag`, all field values) are
**independent for every granule**, so heterogeneous granule mixes are separate
rows: `scfsi`, `part_23_sum` and `gr++` carry state across the `do…while` loop
and that coupling is where composed-pipeline bugs hide.

Every row below is run with **many pseudo-random inputs** (SplitMix64, fixed
per-row seed): all non-fixed side-info fields, all four `hdr` bytes' don't-care
bits, the 128-byte backing buffer, and the `L3_gr_info_t` sentinel fill are
randomized per iteration; both `.so`s are called with identical inputs and the
return value, the post-call `bs` and **all 32 bytes of every** `L3_gr_info_t`
(pointer field compared via cross-granule deltas + pointed-to table contents)
must be identical.

| #  | entry point | configuration (options set + input shape) | [x] |
|----|-------------|-------------------------------------------|-----|
| 1  | `read_side_info` | A=0, mono ⇒ `gr_count=1`, W=0, sr_idx swept 0..5, aligned `pos=0`, ample limit | [x] |
| 2  | `read_side_info` | A=0, ¬mono ⇒ `gr_count=2`, W=0, sr_idx 0..5, ample limit | [x] |
| 3  | `read_side_info` | A=8, mono ⇒ `gr_count=2`, W=0, sr_idx 2..8, ample limit | [x] |
| 4  | `read_side_info` | A=8, ¬mono ⇒ `gr_count=4`, W=0, sr_idx 2..8, ample limit | [x] |
| 5  | `read_side_info` | A=0, mono, W=1, block_type=1 (long table, `region_count={7,255,untouched}`) | [x] |
| 6  | `read_side_info` | A=0, mono, W=1, block_type=3 | [x] |
| 7  | `read_side_info` | A=0, mono, W=1, block_type=2, mixed=0 (short table, `region_count[0]=8`) | [x] |
| 8  | `read_side_info` | A=0, mono, W=1, block_type=2, mixed=1 (mixed table, `n_long_sfb=6`) | [x] |
| 9  | `read_side_info` | A=8, mono, W=1, block_type=1 | [x] |
| 10 | `read_side_info` | A=8, mono, W=1, block_type=3 | [x] |
| 11 | `read_side_info` | A=8, mono, W=1, block_type=2, mixed=0 | [x] |
| 12 | `read_side_info` | A=8, mono, W=1, block_type=2, mixed=1 (`n_long_sfb=8`, the A-dependent branch) | [x] |
| 13 | `read_side_info` | A=0, ¬mono (gr_count=2), W=1 for both granules, all 3×2 block_type/mixed combos per granule | [x] |
| 14 | `read_side_info` | A=8, ¬mono (gr_count=4), W=1 for all granules, block_type/mixed randomized independently per granule | [x] |
| 15 | `read_side_info` | heterogeneous: W randomized per granule (mix of W=0 and W=1 granules), gr_count=4 | [x] |
| 16 | `read_side_info` | A=8, gr_count=4, granule 0 block_type=2 (triggers `scfsi &= 0x0F0F`) and granules 1..3 W=0 ⇒ tests scfsi masking propagation to later `gr->scfsi` | [x] |
| 17 | `read_side_info` | A=8, mono, gr_count=2, `scfsi` field = all 9 bits swept over random values, per-granule `scfsi` nibble extraction (`(scfsi>>12)&15`, extra `<<4` for mono) | [x] |
| 18 | `read_side_info` | A=8, ¬mono, gr_count=4, 11-bit `scfsi` field, no extra `<<4` | [x] |
| 19 | `read_side_info` | A=0 ⇒ `scfsi` never read: `gr->scfsi` must be 0 for every granule, mono and ¬mono | [x] |
| 20 | `read_side_info` | A=0, `scalefac_compress` swept over 0..511 incl. the 499/500 boundary ⇒ `preflag` | [x] |
| 21 | `read_side_info` | A=8, `preflag` read as an explicit bit; `scalefac_compress` only 4 bits | [x] |
| 22 | `read_side_info` | sr_idx = 8 exactly (A=8, hdr[1] bit4 = 1, hdr[2] bits2-3 = 3), W=0 ⇒ `&g_scf_long[8]` OOB row | [x] |
| 23 | `read_side_info` | sr_idx = 8, W=1, block_type=2, mixed=0 ⇒ `&g_scf_short[8]` == `g_scf_mixed[0]` | [x] |
| 24 | `read_side_info` | sr_idx = 8, W=1, block_type=2, mixed=1 ⇒ `&g_scf_mixed[8]` past `.rodata` (contents skipped, everything else compared — see `ERRORS.md` note) | [x] |
| 25 | `read_side_info` | `bs->pos` start alignment swept over all 8 values 0..7 (byte-straddling reads for every field width) × gr_count 1/2/4 | [x] |
| 26 | `read_side_info` | `bs->pos` large but byte-aligned deep inside the buffer (`pos = 8*64`), ample limit | [x] |
| 27 | `read_side_info` | `bs->pos` **negative** (−1 … −64, `pos>>3` arithmetic shift, `pos&7` of a negative), `bs->buf` placed mid-buffer so the read is defined | [x] |
| 28 | `read_side_info` | `bs->limit` exactly equal to the last bit consumed (no truncation, tightest legal limit) | [x] |
| 29 | `read_side_info` | `bs->limit` one bit short of the last bit consumed (only the final read truncates) | [x] |
| 30 | `read_side_info` | `part_23_length` values chosen so `part_23_sum + pos` lands just below / on / just above `limit + mdb*8` | [x] |
| 31 | `read_side_info` | `part_23_length` = 4095 in every granule (max 12-bit value, max `part_23_sum` = 16380) | [x] |
| 32 | `read_side_info` | `big_values` = 0 / 1 / 287 / 288 (all accepted) | [x] |
| 33 | `read_side_info` | all-zero buffer (every field 0 ⇒ W=0, `block_type=0` legally, `main_data_begin=0`) with ample limit | [x] |
| 34 | `read_side_info` | all-`0xFF` buffer (every field at max: `big_values=511` ⇒ error path is row 7 of `ERRORS.md`; with `big_values` forced to ≤288 all other fields max) | [x] |
| 35 | `read_side_info` | `gr` array larger than `gr_count` (6 slots) pre-filled with a random sentinel ⇒ verifies the C writes exactly `gr_count` structs and no more | [x] |
| 36 | `read_side_info` | W=1 path leaves `region_count[2]` at the caller's previous value (sentinel differs per iteration); W=0 path sets it to 255 | [x] |
| 37 | `read_side_info` | `hdr[0]` fully random / irrelevant (never read), `hdr[1..3]` don't-care bits random ⇒ verifies no hidden dependency on unread bits | [x] |
| 38 | `read_side_info` | full random sweep: random `hdr[0..3]`, random 128-byte buffer, random `pos` (0..64), random `limit` (0..8·len) — hits arbitrary combinations incl. truncation, all block types and both error paths | [x] |

## Result

```
$ cargo build && cargo test --test phase_b_configs
running 38 tests
...
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Every row above passes across its randomized inputs, in all three cargo
configurations (`--no-default-features`, default, `--all-features` — identical
by construction, see `SYMBOLS.md`) and against both the dev-profile and the
release-profile Rust `.so` (`scripts/verify.sh`).

## Note on the reference C build

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference `.so` is
built at `-O0`, where gcc emits the three function-local `static const` tables
in declaration order:

```
.rodata +0    g_scf_long   (184 B)
        +184  padding      (8 B, zeros)
        +192  g_scf_short  (320 B)
        +512  g_scf_mixed  (320 B)   -> .rodata ends at +832
```

`src/lib.rs` reproduces exactly this layout, which is what makes rows 22-24
(the `sr_idx == 8` out-of-bounds table rows) byte-identical. For the record,
gcc **reverses** the order at `-O1` and above (`mixed`, `short`, `long`), so the
C's own out-of-bounds reads are not stable across optimisation levels — see the
note at the end of `ERRORS.md`.

## Row → test mapping

| # | test in `tests/phase_b_configs.rs` |
|---|---|
| 1 | `cfg_01_mpeg2_mono_w0` |
| 2 | `cfg_02_mpeg2_stereo_w0` |
| 3 | `cfg_03_mpeg1_mono_w0` |
| 4 | `cfg_04_mpeg1_stereo_w0` |
| 5 | `cfg_05_mpeg2_w1_bt1` |
| 6 | `cfg_06_mpeg2_w1_bt3` |
| 7 | `cfg_07_mpeg2_w1_bt2_short` |
| 8 | `cfg_08_mpeg2_w1_bt2_mixed` |
| 9 | `cfg_09_mpeg1_w1_bt1` |
| 10 | `cfg_10_mpeg1_w1_bt3` |
| 11 | `cfg_11_mpeg1_w1_bt2_short` |
| 12 | `cfg_12_mpeg1_w1_bt2_mixed` |
| 13 | `cfg_13_two_granules_all_block_type_combos` |
| 14 | `cfg_14_four_granules_random_block_types` |
| 15 | `cfg_15_heterogeneous_window_flags` |
| 16 | `cfg_16_scfsi_masking_propagation` |
| 17 | `cfg_17_scfsi_mpeg1_mono_9bit` |
| 18 | `cfg_18_scfsi_mpeg1_stereo_11bit` |
| 19 | `cfg_19_scfsi_absent_on_mpeg2` |
| 20 | `cfg_20_preflag_from_scalefac_compress_mpeg2` |
| 21 | `cfg_21_preflag_explicit_bit_mpeg1` |
| 22 | `cfg_22_sr_idx_8_long_row` |
| 23 | `cfg_23_sr_idx_8_short_row` |
| 24 | `cfg_24_sr_idx_8_mixed_row` |
| 25 | `cfg_25_all_start_alignments` |
| 26 | `cfg_26_deep_byte_aligned_pos` |
| 27 | `cfg_27_negative_pos` |
| 28 | `cfg_28_limit_exactly_at_end` |
| 29 | `cfg_29_limit_one_bit_short` |
| 30 | `cfg_30_part23_sum_boundary` |
| 31 | `cfg_31_part23_length_max` |
| 32 | `cfg_32_big_values_boundaries` |
| 33 | `cfg_33_all_zero_buffer` |
| 34 | `cfg_34_all_ones_buffer` |
| 35 | `cfg_35_no_writes_past_gr_count` |
| 36 | `cfg_36_region_count2_preservation` |
| 37 | `cfg_37_hdr0_and_dontcare_bits_ignored` |
| 38 | `cfg_38_random_hdr_sweep` |
