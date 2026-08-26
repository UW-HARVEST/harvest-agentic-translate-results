# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Public entry points (the FULL set, lowest level included)

| entry point | declared in | signature |
|-------------|-------------|-----------|
| `flac_validate` | `include/lib.h` | `int flac_validate(tflac *t)` |
| `tflac_size_memory` | *(not in the header — exported anyway)* | `tflac_u32 tflac_size_memory(tflac_u32 blocksize)` |

There is no convenience/one-shot wrapper layer: both exported functions *are*
the lowest level. Both are driven directly through the `.so` exports.

## Axes the C code branches on

Every axis below corresponds to a literal `if` / `while` in `src/lib.c`.
There are **no** `switch`es and **no** `#ifdef`s.

| axis | field / argument | states the C distinguishes | source |
|------|------------------|-----------------------------|--------|
| **A** | `t->channel_mode` (`tflac_u8`, *not* an `enum`-typed field) | `0` = `TFLAC_CHANNEL_INDEPENDENT` → branch skipped; **any non-zero** (`1` `LEFT_SIDE`, `2` `SIDE_RIGHT`, `3` `MID_SIDE`, and the out-of-range `4`=`MODE_COUNT` … `255`) → branch taken | line 32 |
| **B** | `t->channels` | `== 2` vs `!= 2` (`1`, `3`…`8`) — decides whether a non-independent mode survives | line 33 |
| **C** | `t->bitdepth` | `== 32` (forces independent) vs `<= 16` (rice auto = `14`) vs `17`…`31` (rice auto = `30`) | lines 33, 38 |
| **D** | `t->max_rice_value` | `== 0` → auto-filled from **C**; `1`…`30` → kept verbatim, auto-fill skipped | lines 37, 43 |
| **E** | `t->min_partition_order` vs `t->max_partition_order` | `min == max` (loop body can never run) vs `min < max` (loop may advance); `max == 0`; `max == 15` (extreme shift `1 << 16`) | lines 49, 52–53 |
| **F** | `t->blocksize` 2-adic valuation | `v2(blocksize)` decides how far the `while` advances `partition_order`: odd (loop never advances), `2^1`·odd, `2^4`·odd, …, `2^15` (`32768`, can saturate at `max`) | line 52 |
| **G** | `t->blocksize` magnitude | in-range `16`…`65535` (boundaries `16`, `17`, `65535`) | lines 16, 18 |
| **H** | `t->samplerate` | in-range `1`…`655350` (boundaries `1`, `655350`) — no other branch depends on it | lines 20, 22 |
| **I** | `t->partition_order`, `t->cur_blocksize` on entry | pure outputs: pre-seeded garbage must be fully overwritten on success | lines 51, 56 |
| **J** | `blocksize` argument of `tflac_size_memory` | `(15 + 4·b) & 0xFFFFFFF0` ⇒ residue of `b mod 4` selects the alignment bucket; `b >= 0x40000000` makes `b * 4U` **wrap** | line 12 |

## Configuration-surface table

One row per combination the C treats differently. Every row is exercised with
**many randomized inputs** (fixed seed, xorshift64\* PRNG) over the free fields,
comparing the `int` return **and** all 28 struct bytes between C and Rust.

### `flac_validate` — channel-mode / channels / bitdepth cross-product (axes A×B×C)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `flac_validate` | `channel_mode = 0` (INDEPENDENT), `channels = 2`, `bitdepth <= 16` — branch skipped, rice auto `14` | [x] |
| 2 | `flac_validate` | `channel_mode = 0`, `channels = 2`, `bitdepth` in `17..=31` — rice auto `30` | [x] |
| 3 | `flac_validate` | `channel_mode = 0`, `channels = 2`, `bitdepth = 32` — rice auto `30`, mode already independent | [x] |
| 4 | `flac_validate` | `channel_mode = 0`, `channels != 2` (`1`,`3`..`8`), all three bitdepth buckets | [x] |
| 5 | `flac_validate` | `channel_mode` in `1..=3`, `channels = 2`, `bitdepth <= 16` — **mode preserved**, rice auto `14` | [x] |
| 6 | `flac_validate` | `channel_mode` in `1..=3`, `channels = 2`, `bitdepth` in `17..=31` — **mode preserved**, rice auto `30` | [x] |
| 7 | `flac_validate` | `channel_mode` in `1..=3`, `channels = 2`, `bitdepth = 32` — **mode forced to 0** (`bitdepth == 32` arm) | [x] |
| 8 | `flac_validate` | `channel_mode` in `1..=3`, `channels != 2`, `bitdepth != 32` — **mode forced to 0** (`channels != 2` arm) | [x] |
| 9 | `flac_validate` | `channel_mode` in `1..=3`, `channels != 2`, `bitdepth = 32` — **both** arms true, mode forced to 0 | [x] |
| 10 | `flac_validate` | `channel_mode` **out of enum range** `4..=255`, `channels = 2`, `bitdepth != 32` — mode **kept at the out-of-range value** | [x] |
| 11 | `flac_validate` | `channel_mode` out of enum range `4..=255`, `channels != 2` or `bitdepth = 32` — mode forced to 0 | [x] |

### `flac_validate` — rice-value axis (axis D × C)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 12 | `flac_validate` | `max_rice_value = 0`, `bitdepth = 1` (lowest) → auto `14` | [x] |
| 13 | `flac_validate` | `max_rice_value = 0`, `bitdepth = 16` (boundary, `<=16`) → auto `14` | [x] |
| 14 | `flac_validate` | `max_rice_value = 0`, `bitdepth = 17` (boundary, `>16`) → auto `30` | [x] |
| 15 | `flac_validate` | `max_rice_value = 0`, `bitdepth = 32` → auto `30` | [x] |
| 16 | `flac_validate` | `max_rice_value` in `1..=30` (randomized), any bitdepth → kept verbatim, no auto-fill | [x] |
| 17 | `flac_validate` | `max_rice_value = 30` exactly (upper valid boundary) | [x] |

### `flac_validate` — partition-order loop (axes E × F × G)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 18 | `flac_validate` | `min = max = 0` — loop body can never run, `partition_order = 0` | [x] |
| 19 | `flac_validate` | `min = max = k` for every `k` in `0..=15` — `partition_order = k` regardless of `blocksize` | [x] |
| 20 | `flac_validate` | `min = 0`, `max = 15`, `blocksize` **odd** (e.g. `17`, `65535`) — loop exits immediately, `partition_order = 0` | [x] |
| 21 | `flac_validate` | `min = 0`, `max = 15`, `blocksize = 2^v · odd` for every `v` in `1..=15` — loop stops exactly at `v` (unclamped) | [x] |
| 22 | `flac_validate` | `min = 0`, `max = 15`, `blocksize = 32768 = 2^15` — loop evaluates the extreme shift `1 << 16` at `partition_order = 15` | [x] |
| 23 | `flac_validate` | `min = 0`, `max < v2(blocksize)` — loop **clamped by `max`** (e.g. `blocksize = 32768`, `max = 3`) | [x] |
| 24 | `flac_validate` | `min > v2(blocksize)`, `min < max` — loop cannot advance, `partition_order = min` (e.g. `blocksize = 17`, `min = 5`, `max = 10`) | [x] |
| 25 | `flac_validate` | `min < max` with `min` inside the divisible run — loop starts mid-run (e.g. `blocksize = 4096`, `min = 2`, `max = 15`) | [x] |
| 26 | `flac_validate` | `max = 15`, `min = 15` — start already at the cap, extreme shift `1 << 16` on the very first test | [x] |
| 27 | `flac_validate` | `blocksize = 16` (lower boundary) with `min = 0, max = 15` | [x] |
| 28 | `flac_validate` | `blocksize = 65535` (upper boundary) with `min = 0, max = 15` | [x] |
| 29 | `flac_validate` | `blocksize = 65536 - 2^k` shapes + all randomized `min<=max` pairs in `0..=15` | [x] |

### `flac_validate` — remaining axes

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 30 | `flac_validate` | `samplerate = 1` and `samplerate = 655350` (both valid boundaries), everything else randomized | [x] |
| 31 | `flac_validate` | `partition_order` / `cur_blocksize` / padding pre-seeded with `0xAA`-style garbage — outputs fully overwritten, padding untouched | [x] |
| 32 | `flac_validate` | **full-random valid structs** (all axes drawn jointly, 2 000 000 iterations, fixed seed) — catches unanticipated axis interactions | [x] |
| 33 | `flac_validate` | **full-random arbitrary 28-byte structs** (valid *and* invalid mixed, 2 x 1 000 000 iterations) — exhaustive joint surface | [x] |
| 34 | `flac_validate` | called **repeatedly on the same struct** (idempotence / state-carry-over: output of run *n* feeds run *n+1*; 100 000 structs x 4 rounds) | [x] |

### `tflac_size_memory` (axis J)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 35 | `tflac_size_memory` | `blocksize = 0` | [x] |
| 36 | `tflac_size_memory` | `blocksize` in `1..=4096` exhaustively — every `mod 4` alignment bucket | [x] |
| 37 | `tflac_size_memory` | `blocksize` over the FLAC-legal range `16..=65535` exhaustively | [x] |
| 38 | `tflac_size_memory` | `blocksize` just below the `* 4U` wrap: `0x3FFFFFFE`, `0x3FFFFFFF` | [x] |
| 39 | `tflac_size_memory` | `blocksize` at/after the `* 4U` wrap: `0x40000000`, `0x40000001`, `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFC`, `0xFFFFFFFF` | [x] |
| 40 | `tflac_size_memory` | randomized full `u32` domain (2 000 000 iterations, fixed seed) | [x] |

## Build configurations

`Cargo.toml` declares **no `[features]`**; `src/` contains **no
`#[cfg(feature = ...)]`**; `c_src/CMakeLists.txt` declares no `option()` and
`src/lib.c` has no `#ifdef`. The complete set of build configurations is
therefore:

| # | feature combination | `cargo check` | Phases B+C |
|---|---------------------|---------------|------------|
| 1 | *(default — empty feature set)* | [x] | [x] |
| 2 | `--no-default-features` (identical to #1, no `default` feature exists) | [x] | [x] |

## Row → test traceability (Phase B)

All tests live in `tests/diff_valid.rs` and run against `tests/common/mod.rs`,
which `dlopen`s the C `.so` **and** both Rust `.so`s and compares the `int`
return value plus all **28 struct bytes (padding included)**.

| CONFIGS row(s) | test function |
|-----------------|---------------|
| 1 | `cfg_row01_indep_2ch_bitdepth_le16` |
| 2 | `cfg_row02_indep_2ch_bitdepth_17_31` |
| 3 | `cfg_row03_indep_2ch_bitdepth_32` |
| 4 | `cfg_row04_indep_not2ch_all_bitdepth_buckets` |
| 5 | `cfg_row05_mode1to3_2ch_bitdepth_le16_mode_preserved` |
| 6 | `cfg_row06_mode1to3_2ch_bitdepth_17_31_mode_preserved` |
| 7 | `cfg_row07_mode1to3_2ch_bitdepth32_mode_forced_indep` |
| 8 | `cfg_row08_mode1to3_not2ch_bitdepth_not32_mode_forced_indep` |
| 9 | `cfg_row09_mode1to3_not2ch_bitdepth32_both_arms` |
| 10 | `cfg_row10_mode_out_of_enum_range_kept` |
| 11 | `cfg_row11_mode_out_of_enum_range_forced_indep` |
| 12, 13, 14, 15 | `cfg_row12to15_rice_autofill_bitdepth_boundaries` |
| 16 | `cfg_row16_rice_1_to_30_kept_verbatim` |
| 17 | `cfg_row17_rice_exactly_30` |
| 18 | `cfg_row18_min_eq_max_eq_zero` |
| 19 | `cfg_row19_min_eq_max_eq_k_for_all_k` |
| 20 | `cfg_row20_odd_blocksize_loop_never_advances` |
| 21 | `cfg_row21_blocksize_2powv_times_odd` |
| 22 | `cfg_row22_blocksize_32768_extreme_shift` |
| 23 | `cfg_row23_loop_clamped_by_max` |
| 24 | `cfg_row24_min_beyond_divisible_run` |
| 25 | `cfg_row25_min_inside_divisible_run` |
| 26 | `cfg_row26_min_eq_max_eq_15_extreme_shift_first_test` |
| 27 | `cfg_row27_blocksize_lower_boundary_16` |
| 28 | `cfg_row28_blocksize_upper_boundary_65535` |
| 29 | `cfg_row29_all_min_max_pairs_x_blocksize_shapes` |
| 30 | `cfg_row30_samplerate_valid_boundaries` |
| 31 | `cfg_row31_output_fields_and_padding_poisoned` |
| 32 | `cfg_row32_full_random_valid_structs` (2 000 000 iterations) |
| 33 | `cfg_row33_full_random_arbitrary_bytes` (2 × 1 000 000 iterations) |
| 34 | `cfg_row34_repeated_calls_feed_forward` (100 000 × 4 rounds) |
| 35 | `cfg_row35_size_memory_zero` |
| 36 | `cfg_row36_size_memory_exhaustive_1_to_4096` |
| 37 | `cfg_row37_size_memory_exhaustive_flac_range` |
| 38 | `cfg_row38_size_memory_just_below_wrap` |
| 39 | `cfg_row39_size_memory_at_and_after_wrap` |
| 40 | `cfg_row40_size_memory_random_full_u32` (2 000 000 iterations) |

### Exhaustive backstops (supersede the sampled rows above)

Because this library's decision space is small, three tests enumerate it
outright rather than sampling it:

| test | space enumerated |
|------|------------------|
| `cfg_exhaustive_blocksize_x_partition_orders` | every `blocksize` in `16..=65535` × every legal `(min_po, max_po)` pair = **8 910 720** configurations (covers rows 18–29 exhaustively) |
| `cfg_exhaustive_mode_x_channels_x_bitdepth_x_rice` | all 256 `channel_mode` × 8 `channels` × 32 `bitdepth` × 31 `max_rice_value` = **2 031 616** configurations (covers rows 1–17 exhaustively) |
| `cfg_exhaustive_size_memory_dense_windows` | `tflac_size_memory` for every value in `0..2^21`, every value within ±4096 of every power of two, and the top 4096 `u32` values (covers rows 35–40) |

Randomized rows use the fixed-seed `xorshift64*` PRNG in `tests/common/mod.rs`
(seeds are literal constants per test), so every run is reproducible. Set
`HARVEST_ITERS=<n>` to scale the randomized row counts for a quick run.
