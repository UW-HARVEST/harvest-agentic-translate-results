# CONFIGS.md — Phase B configuration-surface table

## Public entry points

`nm -D` on the C `.so` exports exactly one function, so the public surface is:

| entry point | linkage | how it is driven |
|-------------|---------|------------------|
| `dequantize_granule(float *grbuf, bs_t *bs, L12_scale_info *sci, int group_size)` | `T` (exported) | called directly through `libloading` |
| `get_bits(bs_t *bs, int n)` | `static` (**lowest-level** routine, not exported) | driven *indirectly but in isolation*: `total_bands = 1`, exactly one non-zero `bitalloc[0] = ba`, `group_size = 1`, so the whole call is a single `get_bits(bs, n(ba))`. Rows G* below are these isolating rows. |

## Axes the C code actually branches on

Derived from the `if`/`while`/`for`/`?` sites in `c_src/src/lib.c`:

| axis | source site | distinct values the code treats differently |
|------|-------------|---------------------------------------------|
| **T** `sci->total_bands` | `i < 2 * sci->total_bands` (line 22) | `0` (loop never runs) · `1` · `2..31` · `32` (`i` reaches 63, last in-bounds `bitalloc`) · `33..64` (`i` ≥ 64 ⇒ reads spill into `scfcod`) · `65..255` (reads spill past the whole struct) |
| **B** `bitalloc[i]` | `if (ba != 0)` / `if (ba < 17)` (24, 25) | `0` (band skipped) · `1` (`half = 0`, 1-bit read) · `2..15` · `16` (`half = 32767`, 16-bit read) · `17` (`mod = 3`) · `18..31` (`mod` grows to `32769`, `n` to `28675`) · `47` (`2 << 30` overflows `int`) · `48` (`2 << 31 == 0` ⇒ `mod == 1`) · `49..80` (shift count masked ⇒ aliases `17..48`) · `255` (`(255-17)&31 == 14`) |
| **G** `group_size` | `k < group_size` (27, 33), `grbuf + group_size*j` (21), `return group_size*4` (42) | `< 0` · `0` · `1` · `2` · `3` · `4` · `12` · `18` · `32` (each changes both the write stride and how many times `code /= mod` runs) |
| **P** `bs->pos & 7` | `s = bs->pos & 7`, `255 >> s`, `bs->buf + (pos>>3)` (4, 6, 9) | `0` (aligned) · `1..7` (unaligned; masks the first byte) |
| **L** `bs->limit` vs reads | `if ((bs->pos += n) > bs->limit)` (7) | limit ≥ every read (fully valid) · limit in the middle (partial underrun) · `pos + n == limit` exactly (last legal read) · `limit == pos` · `limit == 0` · `limit < 0` |
| **S** `shl = n + s` step count | `while ((shl -= 8) > 0)` (10) | 0 loop iterations (`n + s <= 8`) · 1..3 (`n <= 31`) · 4 (`shl` ends exactly at `0` ⇒ `next >> 0`) · many (wide `mod` reads, `n` up to `28675`) |
| **D** `bs->buf` bytes | `next = *p++ & (255 >> s)` (9) | all `0x00` · all `0xFF` (max magnitudes / sign flips) · random |
| **C** `choff` walk | `dst += choff; choff = 18 - choff;` (38, 39) | `+576` then `-558`, alternating, **carried across the `j` loop**; only observable with `total_bands >= 1` and a large `grbuf` |
| **X** `scfcod` / post-struct bytes | out-of-bounds `bitalloc[i]` for `i >= 64` | all `0x00` · random (become bit-allocations!) |

`j` is a fixed `0..4` loop with no data dependence, so it is not an axis; it is
however what makes **C** (the `choff` carry) observable, and all rows below run
the full 4 granules.

## Rows

Every row is run with **many randomized inputs** (fixed-seed PRNG; bitstream
bytes, `scf`, `scfcod`, post-struct padding and — where the row does not pin
them — `pos`, `limit` and the per-band `bitalloc` values are all randomized).
Both libraries are called through their `.so` exports and the full `grbuf`,
the return value, and the mutated `bs` are compared byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | differential test | [x] |
|----|----------------|--------------------------------------------|-------------------|-----|
|G1 | `get_bits` isolated | T=1, one band, B=1 (1-bit read), G=1, P=0, L=generous, D=random | `g1_get_bits_single_bit` | [x] |
|G2 | `get_bits` isolated | T=1, B=1..16 (every `half`-branch width), G=1, P=0, L=generous, D=random | `g2_get_bits_every_half_branch_width_aligned` | [x] |
|G3 | `get_bits` isolated | T=1, B=1..16, G=1, P=1..7 (every unaligned start), L=generous, D=random | `g3_get_bits_every_width_times_every_unaligned_start` | [x] |
|G4 | `get_bits` isolated | T=1, B=16, G=1, P=0..7, S=4 (`shl` lands exactly on 0 ⇒ `next >> 0`), L=generous | `g4_get_bits_shl_lands_exactly_on_zero` | [x] |
|G5 | `get_bits` isolated | T=1, B=17..46 (every `mod` width that does not underrun), G=1, P=0..7, L=generous | `g5_get_bits_mod_branch_widths` | [x] |
|G6 | `get_bits` isolated | T=1, B=47 (`2<<30` `int` overflow ⇒ `mod = 0x80000001`, `n = 0x70000003`), G=1, L=generous-but-finite ⇒ guard fires | `g6_get_bits_ba47_signed_shift_overflow` | [x] |
|G7 | `get_bits` isolated | T=1, B=48 (`2<<31 == 0` ⇒ `mod == 1`, `n == 3`), G=1, P=0..7, L=generous | `g7_get_bits_ba48_mod_is_one` | [x] |
|G8 | `get_bits` isolated | T=1, B=49..255 (shift-count aliasing, period 32), G=1, P=0..7, L=generous | `g8_get_bits_shift_count_aliasing_above_48` | [x] |
|G9 | `get_bits` isolated | T=1, B random 1..255, G=1, L = exactly `pos + n` (last legal read, `>` not `>=`) | `g9_limit_exactly_equals_pos_plus_n_last_legal_read` | [x] |
|G10 | `get_bits` isolated | T=1, B random 1..255, G=1, L = `pos + n - 1` (first illegal read) | `g10_limit_one_below_pos_plus_n_first_illegal_read` | [x] |
|G11 | `get_bits` isolated | T=1, B=1..16, G=1, P=0..7, D = all `0x00` | `g11_all_zero_bitstream` | [x] |
|G12 | `get_bits` isolated | T=1, B=1..16, G=1, P=0..7, D = all `0xFF` (max magnitude, `next` fully set) | `g12_all_ones_bitstream` | [x] |
|G13 | `get_bits` isolated | T=1, B=17..31, G=1, D = all `0xFF` (wide `mod` read, `cache` saturated) | `g13_mod_branch_with_all_ones_bitstream` | [x] |
|G14 | `get_bits` isolated | T=1, B=31 (`n = 28675`, S=many: ~3585 loop steps), G=1, L=generous, D=random | `g14_widest_legal_read_many_loop_steps` | [x] |
|G15 | `get_bits` isolated | T=1, B random, G=1, P = byte-aligned but non-zero (`pos = 8*m`) | `g15_byte_aligned_nonzero_start_positions` | [x] |
|B1 | `dequantize_granule` | T=0 (empty band set), G=4, everything else random | `b1_total_bands_zero` | [x] |
|B2 | `dequantize_granule` | T=1, B=all zero (band skipped, `choff` still walks), G=4 | `b2_all_bitalloc_zero` | [x] |
|B3 | `dequantize_granule` | T=1, B random 1..16, G=1,2,3,4,12,18,32 (write-stride sweep), P=0, L=generous | `b3_group_size_sweep_aligned` | [x] |
|B4 | `dequantize_granule` | T=1, B random 1..16, G as above, P=1..7 (unaligned), L=generous | `b4_group_size_sweep_unaligned` | [x] |
|B5 | `dequantize_granule` | T=2..8, B random 1..16 mixed with zeros, G=4/12, L=generous (multi-band `choff` walk) | `b5_multi_band_choff_walk` | [x] |
|B6 | `dequantize_granule` | T=31, B random 1..16, G=12, L=generous (`i` up to 61, in-bounds `bitalloc`) | `b6_total_bands_31` | [x] |
|B7 | `dequantize_granule` | T=32, B random 1..16 (all 64 `bitalloc` bytes used, `i` max = 63), G=12 | `b7_total_bands_32_uses_all_64_bitalloc_bytes` | [x] |
|B8 | `dequantize_granule` | T=33..64, B random, **X random** ⇒ `bitalloc[64..127]` aliases `scfcod` (out-of-bounds read inside the struct), G=12 | `b8_total_bands_33_to_64_reads_spill_into_scfcod` | [x] |
|B9 | `dequantize_granule` | T=65..255, B random, X random ⇒ reads past the whole `L12_scale_info` into trailing padding, G=4 | `b9_total_bands_above_64_reads_past_the_struct` | [x] |
|B10 | `dequantize_granule` | T=255 (max `uint8_t`), B/X random 1..255, G=4, L modest ⇒ mixture of legal reads and underruns | `b10_total_bands_255_full_value_range` | [x] |
|B11 | `dequantize_granule` | T=2..64, B random **1..255** (mixed `half` and `mod` branches in one call), G=4/12, L=generous | `b11_mixed_half_and_mod_branches_in_one_call` | [x] |
|B12 | `dequantize_granule` | T=2..64, B ∈ {17,18,19,20,48} only (`mod` branch only, `code /= mod` chains), G=12 | `b12_mod_branch_only_division_chains` | [x] |
|B13 | `dequantize_granule` | T=8, B random, G=4, L placed mid-stream ⇒ first bands decode, later bands underrun (latching guard) | `b13_limit_mid_stream_partial_underrun` | [x] |
|B14 | `dequantize_granule` | T=8, B random, G=4, L=0 (nothing readable at all) | `b14_limit_zero` | [x] |
|B15 | `dequantize_granule` | T=8, B random, G=4, L<0 | `b15_negative_limit` | [x] |
|B16 | `dequantize_granule` | T=8, B random, G=4, `pos` already `> limit` on entry | `b16_pos_already_past_limit` | [x] |
|B17 | `dequantize_granule` | T=random, B random, G=0 (no writes but `mod`-branch `get_bits` still consumes bits) | `b17_group_size_zero` | [x] |
|B18 | `dequantize_granule` | T=random, B random, G<0 (`dst` walks *before* `grbuf`, still no writes; negative return) | `b18_negative_group_size` | [x] |
|B19 | `dequantize_granule` | T=random, B random, G=1 (single sample per band ⇒ `code /= mod` runs once) | `b19_group_size_one` | [x] |
|B20 | `dequantize_granule` | T=64, B random 1..255, G=32 (largest stride × widest band set: `choff` walk reaches ≈ 5.1 k floats) | `b20_widest_band_set_times_largest_stride` | [x] |
|B21 | `dequantize_granule` | D = all `0x00`, T random, B random 1..255, G=12 (all-zero bitstream ⇒ `dst = -half` / `-(mod/2)`) | `b21_zero_bitstream_full_range` | [x] |
|B22 | `dequantize_granule` | D = all `0xFF`, T random, B random 1..255, G=12 | `b22_ones_bitstream_full_range` | [x] |
|B23 | `dequantize_granule` | full random fuzz over **all** axes simultaneously (T,B,G,P,L,D,X), 20 000 cases | `b23_full_random_fuzz_over_all_axes` | [x] |
|B24 | `dequantize_granule` | `grbuf` pre-filled with a distinctive pattern; asserts untouched slots stay identical and the *set of touched offsets* matches (`choff` carry across the `j` loop) | `b24_untouched_slots_and_touched_offset_pattern` | [x] |
|B25 | `dequantize_granule` | repeated back-to-back calls on the **same** `bs_t` (state carried between calls: `pos` monotonically advances, later calls underrun) | `b25_chained_calls_share_one_bit_reader` | [x] |
|B26 | `dequantize_granule` | G = 18 / 64 / 128 / **576** (the real MPEG granule width) x T = 1,2,8,32,64,255, B narrow, L=generous — largest write strides, overlapping `j` regions | `b26_large_group_size_full_granule_strides` | [x] |

## Result

All **41** rows pass, each across many fixed-seed randomized inputs, against
both the debug and the release Rust `.so` and against the C `.so` built both at
`-O0` (the default `CMakeLists.txt` build) and at `-O2`.

Run them with:

```
cargo test --offline --test phase_b
bash scripts/check_feature_combos.sh    # every feature combo x profile
```
