# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the branches the C in `c_src/src/lib.c` actually
takes. There are no `#ifdef`s, no build-time options, and the Cargo crate
declares **no** cargo features (`[features]` absent from `Cargo.toml`), so the
only configuration surface is the *runtime state of the `tflac` struct* plus the
`blocksize` argument of `tflac_size_memory`.

## Public entry points (FULL set — both, lowest-level included)

| entry point | signature | header? |
|---|---|---|
| `tflac_size_memory` | `tflac_u32 (tflac_u32 blocksize)` | no — internal-linkage-free, still exported |
| `flac_validate`     | `int (tflac *t)`                  | yes (`include/lib.h`) |

## Axes the C branches on

| axis | values the code distinguishes | C site |
|---|---|---|
| A1 `channel_mode` | `0` (`TFLAC_CHANNEL_INDEPENDENT`) vs non-zero (`1`,`2`,`3`,`4=MODE_COUNT`,`5..=255`) | line 32 |
| A2 `channels` | `== 2` vs `!= 2` (1,3..8) — gates the stereo-mode fix-up | line 33 |
| A3 `bitdepth` | `== 32`, `<= 16`, `17..=31` — gates fix-up AND rice default | lines 33, 38 |
| A4 `max_rice_value` | `== 0` (auto-default) vs `1..=30` (kept) | lines 37, 43 |
| A5 `max_partition_order` | `0`, `1..=14`, `15` | lines 46, 53 |
| A6 `min_partition_order` | `== max` (loop cannot advance) vs `< max` (loop may advance) | lines 49, 51 |
| A7 `blocksize` 2-adic valuation | odd (loop never runs), `v2 == 1`, `v2 >= max_order+1` (loop saturates at max), intermediate `v2` (loop stops early) | line 52 |
| A8 `blocksize` magnitude | `16` (min), interior, `65535` (max, odd), `65534`, `32768` (`v2=15`), `65536-…` | lines 16/18 |
| A9 `samplerate` | `1`, interior, `655350` (max) — no downstream branch, but must round-trip | lines 20/22 |
| A10 `tflac_size_memory` arg | small, values where `blocksize*4` wraps, values where `+15` wraps, `u32::MAX` | line 12 |

## Configuration rows (cross-product, pruned to what C distinguishes)

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `tflac_size_memory` | `blocksize = 0` | `cfg_01_size_memory_zero` | [x] |
| 2  | `tflac_size_memory` | small non-wrapping blocksizes `1..=65535` (exhaustive) | `cfg_02_size_memory_small_exhaustive` | [x] |
| 3  | `tflac_size_memory` | `blocksize` such that `blocksize*4` wraps mod 2^32 (`>= 2^30`), incl. `u32::MAX`, `2^30`, `2^30-1`, `0xC000_0000` | `cfg_03_size_memory_wrapping` | [x] |
| 4  | `tflac_size_memory` | 200 000 uniformly random `u32` (seeded LCG) | `cfg_04_size_memory_random` | [x] |
| 5  | `flac_validate` | A1=0 independent, A4=0, A3<=16, A5=0, A6 min=max=0, blocksize odd | `cfg_05_independent_minimal` | [x] |
| 6  | `flac_validate` | A1=0 independent, A4=0, A3>16 (`17..=31`) → rice default 30 | `cfg_06_rice_default_hi_bitdepth` | [x] |
| 7  | `flac_validate` | A1=0 independent, A4=0, A3=16 boundary → rice default 14 | `cfg_07_rice_default_bitdepth_16` | [x] |
| 8  | `flac_validate` | A1=0 independent, A4=0, A3=17 boundary → rice default 30 | `cfg_07_rice_default_bitdepth_16` | [x] |
| 9  | `flac_validate` | A1=0, A4 in `1..=30` (all values) → kept verbatim, no default applied | `cfg_09_rice_explicit_all_values` | [x] |
| 10 | `flac_validate` | A1!=0 (`1,2,3`) with A2=2, A3<32 → **mode preserved** | `cfg_10_stereo_mode_preserved` | [x] |
| 11 | `flac_validate` | A1!=0 with A2=2, A3=32 → **mode reset to 0** (bitdepth-32 clause) | `cfg_11_stereo_mode_reset_bitdepth32` | [x] |
| 12 | `flac_validate` | A1!=0 with A2 != 2 (`1,3,4,5,6,7,8`), A3<32 → **mode reset to 0** (channel clause) | `cfg_12_mode_reset_wrong_channels` | [x] |
| 13 | `flac_validate` | A1 out-of-range (`4=MODE_COUNT`, `5`, `127`, `128`, `255`) × A2 ∈ {2, other} × A3 ∈ {16, 32} | `cfg_13_mode_out_of_range_matrix` | [x] |
| 14 | `flac_validate` | A7 odd blocksize (e.g. 65535, 4097, 17) with A5=15, A6 min=0 → loop never advances, `partition_order == min` | `cfg_14_odd_blocksize_no_advance` | [x] |
| 15 | `flac_validate` | A7 `v2(blocksize) == 1` (e.g. 4098) with A5=15 → loop advances exactly once | `cfg_15_v2_one` | [x] |
| 16 | `flac_validate` | A7 `v2 >= max_order+1` (e.g. blocksize 32768, A5=15) → loop **saturates** at `max_partition_order` | `cfg_16_loop_saturates_at_max` | [x] |
| 17 | `flac_validate` | A7 intermediate `v2` (blocksize 4096 `v2=12`, A5=15) → loop stops at `v2`, below max | `cfg_17_loop_stops_below_max` | [x] |
| 18 | `flac_validate` | A6 `min == max` for every value `0..=15`, blocksize highly divisible → loop cannot advance despite divisibility | `cfg_18_min_eq_max_all` | [x] |
| 19 | `flac_validate` | A6 `min < max`, full matrix `min ∈ 0..=15`, `max ∈ min..=15`, blocksize ∈ {16, 32768, 65535, 4096} | `cfg_19_partition_order_matrix` | [x] |
| 20 | `flac_validate` | A8 blocksize at boundaries 16 and 65535 and 65534, with A5=15 | `cfg_20_blocksize_boundaries` | [x] |
| 21 | `flac_validate` | A9 samplerate 1 / 44100 / 655350, A2 all `1..=8`, A3 all `1..=32` (full valid cross-product) | `cfg_21_full_valid_cross_product` | [x] |
| 22 | `flac_validate` | garbage in the 3 padding bytes at offsets 21..24 on a **valid** input (must round-trip untouched) | `cfg_22_padding_untouched_valid` | [x] |
| 23 | `flac_validate` | 300 000 fully random 28-byte struct images (seeded LCG) covering valid+invalid at once; compare return code **and** all 28 bytes | `cfg_23_random_fuzz_full_struct` | [x] |
| 24 | `flac_validate` | 200 000 random structs drawn from the **valid** ranges only (so the success path with its 4 mutations is hit densely) | `cfg_24_random_fuzz_valid_only` | [x] |
| 25 | `flac_validate` | repeated / idempotent invocation: call twice on the same struct, compare after each call (state carried between calls) | `cfg_25_repeated_invocation` | [x] |

## Binary executable

The project builds **no** binary/driver: `c_src/CMakeLists.txt` declares only
`add_library(... SHARED src/lib.c)`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]` and no `src/main.rs`.
The "compare C and Rust binary stdout" gate is therefore **not applicable**.

## Feature combinations

`translation/Cargo.toml` has **no** `[features]` section, so the only
configuration is the default one. `cargo test --no-default-features` is still
run in Phase D to confirm it is identical.
