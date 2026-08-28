# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

Derived mechanically from the branches the C actually takes.

## Axes found in the C source

**Public entry points** (all three are exported by the `.so`; two of them are
the *low-level* primitives that `lib.h` does not even declare, and they are
tested directly, not only through the `update_md5` wrapper):

| entry point | source | role |
|---|---|---|
| `tflac_pack_u64le(d, n)` | src/lib.c:5 | lowest level: 8 LE byte stores |
| `tflac_md5_addsample(m, bits, val)` | src/lib.c:16 | mid level: MD5 buffer state machine |
| `update_md5(t, samples)` | src/lib.c:33 | top level: 5-iteration sample packer |

**Runtime options / mode flags:** the API has no flags, no enums, no setters.
The only "configuration" is the mutable state inside `tflac` / `tflac_md5`
plus the integer arguments — those *are* the option axes:

| axis | values the C distinguishes | why (branch in the C) |
|---|---|---|
| `m->pos` (entry state) | `0`; `1..7` (not byte-of-8 aligned); `8`; `56` (`pos+8 == 64` exactly); `57..63` (spill 1..7); `63` (last in-range store, writes `buffer[63..71]`); `>= 64` (never sanitised); `u32::MAX` (wrap) | `pos2 = pos % 64`, `if (m->pos >= 64)`, `bytes = m->pos`, spill loop count |
| `bits` | `0`; `8,16,24,32,40,48,56` (sub-word widths); `64` (the only value `update_md5` uses); non-multiples of 8 (`1..7, 9, 63, 65`); huge (`1024`, `u32::MAX`) | `total += bits`, `bytes = bits / 8` (truncating), then `pos >= 64` |
| `m->total` (entry state) | `0`; mid; near `u64::MAX` (wraps) | `m->total += bits` |
| `val` | `0`; `u64::MAX`; single-byte patterns; random | 8 independent byte stores |
| `m->buffer` tail (`buffer[64..72]`) | distinct pattern vs. zeros | spill loop copies tail → head |
| `t->cur_blocksize`, `t->channels` | product `== 40` (returns 0); `< 40` incl. `0` (u32 underflow); `> 40`; product overflowing u32 | `b = cur_blocksize * channels`, `b -= 8` ×5 |
| sample values | `0`; `-1`; `i32::MIN`; `i32::MAX`; low byte `0x00`/`0xFF`; sign-mixed random | `((tflac_uint)samples[k]) & 0xFF` (sign-extend then mask) |
| sample layout | which indices are read: `0..8`, `32..40`, `64..72`, `96..104`, `128..136` (stride is `8*sizeof(tflac_s32)` = **32 elements**) | `samples += (8 * sizeof(tflac_s32))` |
| call sequencing | single call vs. long randomized sequence on the same struct (state machine) | all state lives in `tflac_md5` |
| destination alignment (`pack_u64le`) | 8-aligned, offsets `1..7`, tail offset 63 | plain byte stores, no alignment branch |

**Build configurations:** none — no `[features]` in `Cargo.toml`, no
`#[cfg(feature)]`, no `#ifdef` in the C. Default == `--no-default-features`
(both are still executed by `run_all.sh`).

## Configuration rows

Every row is exercised against **both** `.so`s with many randomized inputs
(deterministic `splitmix64`, fixed seeds) and compared byte-for-byte: return
value **and** the entire 512-byte arena holding the struct (so stray writes and
the out-of-object spill reads are both caught).

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|--------------------------------------------|------|---|
| 1 | `tflac_pack_u64le` | 8-aligned destination, 4096 randomized `n` | `cfg01_pack_aligned` | [x] |
| 2 | `tflac_pack_u64le` | destination offsets 1..7 (unaligned), randomized `n` | `cfg02_pack_unaligned` | [x] |
| 3 | `tflac_pack_u64le` | boundary `n`: 0, 1, `u64::MAX`, `0x8000…`, one-byte-set patterns, byte-ramp | `cfg03_pack_boundary_values` | [x] |
| 4 | `tflac_pack_u64le` | destination at buffer tail (offset 63 of a 72-byte buffer → stores 63..70) | `cfg04_pack_tail_offset` | [x] |
| 5 | `tflac_md5_addsample` | `pos = 0`, `bits = 64`, randomized `val`/buffer/total | `cfg05_addsample_pos0_bits64` | [x] |
| 6 | `tflac_md5_addsample` | `pos = 56`, `bits = 64` → `pos` hits exactly 64, spill count 0 (boundary, loop skipped) | `cfg06_addsample_exact_boundary` | [x] |
| 7 | `tflac_md5_addsample` | `pos = 57..63`, `bits = 64` → spill of 1..7 bytes, tail pre-filled with a distinct pattern | `cfg07_addsample_spill_partial` | [x] |
| 8 | `tflac_md5_addsample` | `pos = 1..7` (unaligned), `bits = 64` | `cfg08_addsample_unaligned_pos` | [x] |
| 9 | `tflac_md5_addsample` | `pos = 63` (largest in-range store: `buffer[63..71]`) | `cfg09_addsample_pos63` | [x] |
| 10 | `tflac_md5_addsample` | `bits = 0` (no advance, but still an 8-byte store) × `pos` sweep 0..63 | `cfg10_addsample_bits0_sweep` | [x] |
| 11 | `tflac_md5_addsample` | sub-word widths `bits ∈ {8,16,24,32,40,48,56}` × full `pos` sweep 0..63 | `cfg11_addsample_bit_width_sweep` | [x] |
| 12 | `tflac_md5_addsample` | multi-block crossing `bits ∈ {512, 1024, 4096}` (`bytes` = 64,128,512) | `cfg12_addsample_multi_block_cross` | [x] |
| 13 | `tflac_md5_addsample` | `total` near `u64::MAX` (wrap) × randomized `bits` | `cfg13_addsample_total_wrap` | [x] |
| 14 | `tflac_md5_addsample` | buffer + tail filled with position-distinct pattern so every spilled byte is identifiable | `cfg14_addsample_spill_source_pattern` | [x] |
| 15 | `tflac_md5_addsample` | `pos ≥ 64` on entry (`64, 65, 71, 127, 128, 1000, 0xFFFF`) | `cfg15_addsample_pos_ge_64` | [x] |
| 16 | `tflac_md5_addsample` | `val` boundaries (0, `u64::MAX`, one-byte patterns) × `pos ∈ {0,7,56,63}` | `cfg16_addsample_val_boundaries` | [x] |
| 17 | `tflac_md5_addsample` | **stateful**: 512 randomized calls on one struct, compared after every call | `cfg17_addsample_sequence_stateful` | [x] |
| 18 | `update_md5` | typical encoder config `cur_blocksize=4096`, `channels=2`, `pos=0`, randomized samples | `cfg18_update_typical` | [x] |
| 19 | `update_md5` | matrix `channels ∈ 1..8` × `cur_blocksize ∈ {1,16,576,4096,65535}` | `cfg19_update_blocksize_channel_matrix` | [x] |
| 20 | `update_md5` | product exactly 40 (`b` ends at 0) — `8×5`, `40×1`, `20×2`, `10×4` | `cfg20_update_product_exact_40` | [x] |
| 21 | `update_md5` | product `< 40` (`0, 1, 8, 39`) → return value underflows | `cfg21_update_product_underflow` | [x] |
| 22 | `update_md5` | product overflows u32 (`0x1000_0000×0x11`, `0xFFFF_FFFF×3`, `65537×65537`) | `cfg22_update_product_overflow` | [x] |
| 23 | `update_md5` | sample value shapes: all 0, all −1, `i32::MIN`, `i32::MAX`, low byte 0x00/0xFF, sign-mixed random | `cfg23_update_sample_value_shapes` | [x] |
| 24 | `update_md5` | sentinel values placed in the *skipped* index ranges (8..32, 40..64, …) to pin the 32-element stride | `cfg24_update_stride_skip` | [x] |
| 25 | `update_md5` | initial `pos` sweep 0..63 (each value changes which of the 5 inner `addsample` calls spills) | `cfg25_update_pos_sweep` | [x] |
| 26 | `update_md5` | initial `pos ≥ 64` combined with `total` near `u64::MAX` | `cfg26_update_pos_ge64_total_wrap` | [x] |
| 27 | `update_md5` | **stateful**: 128 randomized rounds on the same `tflac`, fresh random samples each round | `cfg27_update_sequence_stateful` | [x] |
| 28 | all three | **composed pipeline**: randomly interleave `pack_u64le`, `addsample` and `update_md5` on the same arena, 512 steps | `cfg28_mixed_pipeline` | [x] |
| 29 | `tflac_md5_addsample`, `update_md5` | struct pointer misaligned by 1..8 bytes × sample pointer misaligned by 0..7 bytes (a `Vec<u8>`-backed caller) | `err_misaligned_struct_pointer` (in `phase_c_errors.rs`) | [x] |

## Result

All 29 rows pass, with the randomized inputs above, against **both** `.so`s in
every profile (`dev`, `release`) and every feature set (`default`,
`--no-default-features`, `--all-features`) — see `run_all.sh`.

Test sensitivity is proven by `mutation_check.sh`: 15 deliberate one-line bugs
injected into `src/lib.rs` (wrong loop count, wrong stride, wrong modulus,
wrong shift, wrong spill offset, swapped fields, …) are **all** detected by
this suite.
