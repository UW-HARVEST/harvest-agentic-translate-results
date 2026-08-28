# CONFIGS.md — Phase A: configuration-surface table (VALID inputs)

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Public entry points (the FULL set — both exported symbols, no wrappers omitted)

| entry point | signature | source |
|---|---|---|
| `tflac_size_memory` | `tflac_u32 (tflac_u32 blocksize)` | `src/lib.c:11` (exported, not in the header — still ABI) |
| `flac_validate` | `int (tflac *t)` | `src/lib.c:15`, declared `include/lib.h:20` |

There is no init/one-shot/convenience layering in this library: both symbols
*are* the lowest level. `flac_validate` is driven the way a real consumer does
it — populate every one of the 10 `tflac` fields (including the output fields
`partition_order` / `cur_blocksize`, pre-seeded with garbage), call, then
compare the return value **and** all 28 struct bytes.

## Axes the C code branches on

**A. `tflac_size_memory` (pure arithmetic, `unsigned int` wraparound):**
* A1 `blocksize == 0`
* A2 `15 + blocksize*4` — value of the low 4 bits before the `& 0xFFFFFFF0` mask (residue of `blocksize` mod 4 decides whether the mask truncates)
* A3 `blocksize * 4` overflows 2^32 (`blocksize >= 0x40000000`)
* A4 `5 * masked` overflows 2^32
* A5 whole 2^32 domain

**B. `flac_validate` runtime "options"/mode fields (all settable by the caller):**
* B1 `channel_mode`: `0` (INDEPENDENT) · `1` (LEFT_SIDE) · `2` (SIDE_RIGHT) · `3` (MID_SIDE) · `4` (MODE_COUNT, no real mode) · `5..255` (no valid variant)
* B2 `max_rice_value`: `0` (auto-derive) · `1..30` (caller-supplied, kept)
* B3 `min_partition_order` / `max_partition_order` pair: `min == max` (loop cannot advance) · `min < max` (loop may advance)

**C. `flac_validate` input-shape axes:**
* C1 `channels`: `== 2` vs `!= 2` (`1`, `3..8`) — gates the channel-mode reset
* C2 `bitdepth`: `== 32` vs `< 32` — gates the channel-mode reset; `<= 16` vs `> 16` — gates the `max_rice_value` default (14 vs 30)
* C3 `blocksize` 2-adic valuation (`v2`), i.e. how many times `blocksize % (1 << (po+1)) == 0` holds: odd (`v2 = 0`) · `16` (`v2 = 4`, min legal) · `4096` (`v2 = 12`) · `32768` (`v2 = 15`, max power of two ≤ 65535) · `65535` (odd, max legal) · arbitrary
* C4 boundary values of every range: blocksize `16` / `65535`; samplerate `1` / `655350`; channels `1` / `8`; bitdepth `1` / `32`; max_rice_value `1` / `30`; partition orders `0` / `15`
* C5 pre-existing garbage in the output fields (`partition_order`, `cur_blocksize`) and in padding bytes 21..23

## Rows (pruned cross-product — one row per combination the C distinguishes)

Every row is exercised with **many randomized inputs** (fixed seed, xorshift64\*
PRNG, ≥256 cases/row unless stated) and asserts C vs Rust equality of the
return value **and** the full 28-byte struct image.

### `tflac_size_memory`

| # | entry point | configuration (options set + input shape) | test | [ ] |
|---|-------------|-------------------------------------------|------|-----|
| 1 | `tflac_size_memory` | A1: `blocksize == 0` | `cfg_01_size_memory_zero` | [x] |
| 2 | `tflac_size_memory` | A2: every `blocksize` in `1..=4096` exhaustively (all residues mod 4/mod 16 of the pre-mask value) | `cfg_02_size_memory_small_exhaustive` | [x] |
| 3 | `tflac_size_memory` | A2 + legal FLAC range: randomized `blocksize ∈ 16..=65535` | `cfg_03_size_memory_legal_range_random` | [x] |
| 4 | `tflac_size_memory` | A4: `5 * masked` wraps but `blocksize*4` does not (`blocksize ∈ 0x0CCCCCCD..0x3FFFFFFB`, randomized) | `cfg_04_size_memory_mul5_overflow` | [x] |
| 5 | `tflac_size_memory` | A3: `blocksize*4` itself wraps (`blocksize ∈ 0x40000000..=0xFFFFFFFF`, randomized + the 4 values around `0x3FFFFFFC`) | `cfg_05_size_memory_mul4_overflow` | [x] |
| 6 | `tflac_size_memory` | A5: whole 2^32 domain swept with a large odd stride (≈300k samples) + `u32::MAX` | `cfg_06_size_memory_full_domain_sweep` | [x] |

### `flac_validate` — channel-mode decision matrix (B1 × C1 × C2)

| # | entry point | configuration (options set + input shape) | test | [ ] |
|---|-------------|-------------------------------------------|------|-----|
| 7  | `flac_validate` | `channel_mode = 0`, `channels = 2`, `bitdepth < 32` → stays 0 | `cfg_07_mode_independent_stereo` | [x] |
| 8  | `flac_validate` | `channel_mode = 0`, `channels != 2` (1,3..8) → stays 0 | `cfg_08_mode_independent_nonstereo` | [x] |
| 9  | `flac_validate` | `channel_mode ∈ {1,2,3}`, `channels = 2`, `bitdepth < 32` → **preserved** | `cfg_09_mode_stereo_preserved` | [x] |
| 10 | `flac_validate` | `channel_mode ∈ {1,2,3}`, `channels = 2`, `bitdepth == 32` → reset to 0 | `cfg_10_mode_reset_by_bitdepth32` | [x] |
| 11 | `flac_validate` | `channel_mode ∈ {1,2,3}`, `channels ∈ {1,3,4,5,6,7,8}` → reset to 0 | `cfg_11_mode_reset_by_channels` | [x] |
| 12 | `flac_validate` | `channel_mode ∈ 4..=255` (no valid variant), `channels = 2`, `bitdepth < 32` → **preserved verbatim** | `cfg_12_mode_out_of_range_preserved` | [x] |
| 13 | `flac_validate` | `channel_mode ∈ 4..=255`, `channels != 2` or `bitdepth == 32` → reset to 0 | `cfg_13_mode_out_of_range_reset` | [x] |

### `flac_validate` — `max_rice_value` derivation (B2 × C2)

| # | entry point | configuration (options set + input shape) | test | [ ] |
|---|-------------|-------------------------------------------|------|-----|
| 14 | `flac_validate` | `max_rice_value = 0`, `bitdepth ∈ 1..=16` → becomes 14 | `cfg_14_rice_auto_14` | [x] |
| 15 | `flac_validate` | `max_rice_value = 0`, `bitdepth ∈ 17..=32` → becomes 30 | `cfg_15_rice_auto_30` | [x] |
| 16 | `flac_validate` | `max_rice_value ∈ 1..=30` (incl. boundaries 1 and 30), any bitdepth → preserved | `cfg_16_rice_explicit_preserved` | [x] |
| 17 | `flac_validate` | `max_rice_value = 0` with `bitdepth == 16` / `== 17` (the exact `<= 16` boundary) | `cfg_17_rice_boundary_16_17` | [x] |

### `flac_validate` — partition-order loop (B3 × C3)

| # | entry point | configuration (options set + input shape) | test | [ ] |
|---|-------------|-------------------------------------------|------|-----|
| 18 | `flac_validate` | `min == max` for every value `0..=15` (loop cannot advance; divisibility still evaluated first) | `cfg_18_partition_min_eq_max` | [x] |
| 19 | `flac_validate` | `min = 0`, `max = 15`, `blocksize = 32768` (`v2 = 15`) → loop advances to 15 | `cfg_19_partition_pow2_blocksize` | [x] |
| 20 | `flac_validate` | `min = 0`, `max = 15`, `blocksize` odd (17, 4097, 65535) → loop never advances | `cfg_20_partition_odd_blocksize` | [x] |
| 21 | `flac_validate` | `min = 0`, `max = 15`, `blocksize = 4096` (`v2 = 12`) → stops at 12 | `cfg_21_partition_stops_at_v2` | [x] |
| 22 | `flac_validate` | `min = 0`, `max = 15`, `blocksize = 16` (`v2 = 4`, min legal) → stops at 4 | `cfg_22_partition_blocksize_16` | [x] |
| 23 | `flac_validate` | full cross-product of `min ∈ 0..=15` × `max ∈ min..=15` × `blocksize ∈ {16,17,24,32,48,96,4096,32768,49152,65534,65535}` | `cfg_23_partition_full_cross` | [x] |
| 24 | `flac_validate` | randomized `min`/`max` (valid pairs) × randomized `blocksize ∈ 16..=65535` | `cfg_24_partition_random` | [x] |

### `flac_validate` — value boundaries & whole-struct fuzz (C4 × C5)

| # | entry point | configuration (options set + input shape) | test | [ ] |
|---|-------------|-------------------------------------------|------|-----|
| 25 | `flac_validate` | samplerate boundaries `1`, `2`, `655349`, `655350`; blocksize `16`/`65535`; channels `1`..`8`; bitdepth `1`..`32` exhaustively | `cfg_25_valid_boundaries` | [x] |
| 26 | `flac_validate` | output fields pre-seeded with garbage (`partition_order = 0xEE`, `cur_blocksize = 0xDEADBEEF`) + padding bytes 21..23 = `0xAA`, on the success path | `cfg_26_output_fields_and_padding_garbage` | [x] |
| 27 | `flac_validate` | fully randomized **valid** structs (all 10 fields drawn from their legal ranges), 20 000 cases | `cfg_27_random_valid_fuzz` | [x] |
| 28 | `flac_validate` | fully randomized **arbitrary** 28-byte struct images (valid *and* invalid mixed, all 256 `channel_mode` values reachable), 50 000 cases | `cfg_28_random_raw_fuzz` | [x] |
| 29 | `flac_validate` | called repeatedly (3×) on the same struct so later calls observe earlier in-place mutations | `cfg_29_repeated_calls` | [x] |
| 30 | `flac_validate` + `tflac_size_memory` | composed pipeline: validate, then feed the resulting `cur_blocksize` into `tflac_size_memory`, randomized | `cfg_30_pipeline_validate_then_size` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the complete set
of feature combinations is: `{default (empty)}`, `{--no-default-features}`.
Both are run by `run_all.sh` (see Phase D notes in `SYMBOLS.md`).

## Phase B result

All 30 rows pass (`tests/phase_b_configs.rs`, 30 tests, ~300 000 differential
calls with fixed PRNG seeds) under every entry of the matrix
`{default, --no-default-features} x {rust debug, rust release} x {CMake C build, -O2 C build}`.

Additional exhaustive confirmation (`tests/phase_e_exhaustive.rs`,
`EXHAUSTIVE=1 ./run_all.sh`, also green on all four library pairings):

| sweep | cases | result |
|---|---|---|
| `tflac_size_memory` over **all 2^32** `blocksize` values | 4 294 967 296 | identical |
| `flac_validate`: `blocksize 0..=70000` x `(min,max) partition order 0..=16` | 20 230 289 | identical |
| `flac_validate`: all `(channel_mode, max_rice_value)` byte pairs x `channels 0..=9` x `bitdepth 0..=33` | 22 282 240 | identical |
| `flac_validate`: all 65 536 `(min,max)` partition-order **byte** pairs x 6 blocksizes | 393 216 | identical |
