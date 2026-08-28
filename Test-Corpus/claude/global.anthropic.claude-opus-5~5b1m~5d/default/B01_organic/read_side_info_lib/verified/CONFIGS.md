# CONFIGS.md — Phase A configuration surface table

## Public entry points

| entry point | linkage | how it is driven |
|-------------|---------|------------------|
| `read_side_info(bs_t*, L3_gr_info_t*, const uint8_t*)` | `T` (exported) | called directly through both `.so`s via `libloading` |
| `get_bits(bs_t*, int n)` | `t` (`static`, **not** exported) | the lowest-level routine; not reachable across the FFI boundary, so it is driven *indirectly and exhaustively* — every distinct `n` the caller can pass (`1,2,3,4,8,9,10,11,12,15`) crossed with every start alignment `s = pos & 7 ∈ 0..=7`, plus its overrun path (see `ERRORS.md` E1/E2) |

## Axes the C code actually branches on

Derived from every `if` / `? :` / loop condition in `c_src/src/lib.c`:

| axis | source | states |
|------|--------|--------|
| A1 `mpeg1` | `hdr[1] & 0x8` (l.91, 110, 131, 152) | 0 / 1 — controls `gr_count *= 2`, `main_data_begin` width (9 vs `8+gr_count` then `>> gr_count`), whether `scfsi` is read at all, `scalefac_compress` width (4 vs 9), mixed `n_long_sfb` (8 vs 6), `preflag` source (bitstream vs `>= 500`) |
| A2 `mono` | `(hdr[3] & 0xC0) == 0xC0` (l.90, 99) | 0 / 1 — base `gr_count` 1 vs 2, and the extra `scfsi <<= 4` at the top of each loop iteration |
| A3 `gr_count` | derived from A1×A2 | 1 (mono+mpeg2), 2 (stereo+mpeg2), 2 (mono+mpeg1), 4 (stereo+mpeg1) |
| A4 `sr_idx` | `((hdr[2]>>2)&3) + (((hdr[1]>>3)&1)+((hdr[1]>>4)&1))*3`, then `-= (sr_idx != 0)` (l.87–89) | `0..=8` — **8 is one row past the end of all three 8-row tables** |
| A5 `window_switching` | `if (get_bits(bs, 1))` (l.114) | 0 / 1 — completely different field layout per granule |
| A6 `block_type` | `get_bits(bs, 2)` (l.115), `== 2` (l.122) | 1 / 2 / 3 (`0` → error E6) |
| A7 `mixed_block_flag` | `get_bits(bs, 1)` (l.119), `!gr->mixed_block_flag` (l.124) | 0 / 1 — selects `g_scf_short` vs `g_scf_mixed` |
| A8 `sfbtab` table | A5/A6/A7 | `g_scf_long` / `g_scf_short` / `g_scf_mixed` |
| A9 start alignment | `s = bs->pos & 7` (l.4) | `0..=7` |
| A10 `scalefac_compress >= 500` | l.152 | 0 / 1 (reachable only when A1=0, where the field is 9 bits) |
| A11 bit budget | `bs->limit` vs consumed bits | ample / exact / truncated mid-parse |
| A12 `main_data_begin` reservoir | l.159 | check passes / fails |
| A13 per-granule shape | the `do{}while(--gr_count)` loop | all granules identical / each granule a different A5–A7 combination |

## Configuration table

One row per combination the C treats differently. Every row is driven with
**many randomized field values** (fixed-seed PRNG, `N` iterations per row) —
never a single hand-picked input. `[x]` = passes byte-for-byte against the C.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1  | `read_side_info` | A1=0 mpeg2, A2=1 mono, gr_count=1, A5=0 non-window-switching (long block, `region_count` from bitstream, `region_count[2]=255`) | `cfg_c1_mpeg2_mono_long` | [x] |
| C2  | `read_side_info` | A1=0, mono, gr_count=1, A5=1, A6=1 (`block_type=1`, `region_count={7,255,retained}`, `sfbtab`=long) | `cfg_c2_mpeg2_mono_ws_bt1` | [x] |
| C3  | `read_side_info` | A1=0, mono, gr_count=1, A5=1, A6=2, A7=0 → `g_scf_short`, `n_long_sfb=0`, `n_short_sfb=39`, `region_count[0]=8`, `scfsi &= 0x0F0F` | `cfg_c3_mpeg2_mono_short` | [x] |
| C4  | `read_side_info` | A1=0, mono, gr_count=1, A5=1, A6=2, A7=1 → `g_scf_mixed`, `n_long_sfb=6` (mpeg2), `n_short_sfb=30` | `cfg_c4_mpeg2_mono_mixed` | [x] |
| C5  | `read_side_info` | A1=0, mono, gr_count=1, A5=1, A6=3 (`sfbtab`=long, `region_count={7,255,retained}`) | `cfg_c5_mpeg2_mono_ws_bt3` | [x] |
| C6  | `read_side_info` | A1=0, A2=0 stereo, gr_count=2, both granules A5=0 | `cfg_c6_mpeg2_stereo_long` | [x] |
| C7  | `read_side_info` | A1=0, stereo, gr_count=2, A13: granule 0 A5=0, granule 1 A5=1/A6=2/A7=0 | `cfg_c7_mpeg2_stereo_mixed_shapes` | [x] |
| C8  | `read_side_info` | A1=1 mpeg1, A2=1 mono, gr_count=2, A5=0 both granules; `main_data_begin`=9 bits, `scfsi`=`get_bits(7+2)`=9 bits | `cfg_c8_mpeg1_mono_long` | [x] |
| C9  | `read_side_info` | A1=1, mono, gr_count=2, A5=1, A6=2, A7=1 → `g_scf_mixed` with `n_long_sfb=8` (the mpeg1-only value) | `cfg_c9_mpeg1_mixed_n_long_sfb_8` | [x] |
| C10 | `read_side_info` | A1=1, A2=0 stereo, gr_count=4, A5=0 all four granules; `scfsi`=`get_bits(7+4)`=11 bits | `cfg_c10_mpeg1_stereo_four_granules` | [x] |
| C11 | `read_side_info` | A1=1, stereo, gr_count=4, A13: a different A5/A6/A7 combination in each of the 4 granules | `cfg_c11_mpeg1_stereo_per_granule_shapes` | [x] |
| C12 | `read_side_info` | A4 sweep `sr_idx = 0..=7` × A8=long (`n_long_sfb=22`, `n_short_sfb=0`) | `cfg_c12_c15_sr_idx_sweep_long` | [x] |
| C13 | `read_side_info` | A4 sweep `sr_idx = 0..=7` × A8=short | `cfg_c13_c16_sr_idx_sweep_short` | [x] |
| C14 | `read_side_info` | A4 sweep `sr_idx = 0..=7` × A8=mixed | `cfg_c14_c17_sr_idx_sweep_mixed` | [x] |
| C15 | `read_side_info` | A4=8 (out of range) × A8=long → `g_scf_long[8]`, one row past the array; aliases the 8 alignment pad bytes + `g_scf_short[0]` in `.rodata` | `cfg_c12_c15_sr_idx_sweep_long` | [x] |
| C16 | `read_side_info` | A4=8 × A8=short → `g_scf_short[8]` aliases `g_scf_mixed[0]` | `cfg_c13_c16_sr_idx_sweep_short` | [x] |
| C17 | `read_side_info` | A4=8 × A8=mixed → `g_scf_mixed[8]` runs off the end of `.rodata` entirely (see note below) | `cfg_c14_c17_sr_idx_sweep_mixed` | [x] |
| C18 | `read_side_info` | A9 sweep: initial `bs->pos` = `0..=63` so every `s = pos & 7` and every byte offset is exercised, × both A5 branches | `cfg_c18_start_alignment_sweep` | [x] |
| C19 | `read_side_info` | A1=0 with `scalefac_compress >= 500` (A10=1) → `preflag = 1` | `cfg_c19_c20_preflag_from_scalefac_compress` | [x] |
| C20 | `read_side_info` | A1=0 with `scalefac_compress < 500` (A10=0) → `preflag = 0`, incl. the `499`/`500` boundary pair | `cfg_c19_c20_preflag_from_scalefac_compress` | [x] |
| C21 | `read_side_info` | `big_values` swept over the whole valid range `0..=288` (A12 kept satisfiable) | `cfg_c21_big_values_full_valid_range` | [x] |
| C22 | `read_side_info` | A1=1, A2=1: `scfsi` shifted left 4 **twice** per iteration (top-of-loop + bottom-of-loop) → `gr->scfsi` propagation across 2 granules | `cfg_c22_scfsi_mono_double_shift` | [x] |
| C23 | `read_side_info` | A1=1, A2=0: 11-bit `scfsi` distributed over 4 granules, one `<<= 4` per iteration, incl. the `scfsi &= 0x0F0F` masking when a granule has A6=2 | `cfg_c23_scfsi_stereo_and_mask` | [x] |
| C24 | `read_side_info` | A1=0 (any A2): `scfsi` is never read → `gr->scfsi == 0` for every granule regardless of bitstream content | `cfg_c24_mpeg2_scfsi_always_zero` | [x] |
| C25 | `read_side_info` | A11=exact: `bs->limit` set to exactly the last bit consumed | `cfg_c25_c27_main_data_begin_reservoir_boundary` | [x] |
| C26 | `read_side_info` | A11=truncated: `bs->limit` swept across every bit position inside the side-info, so the overrun happens at every distinct field | `cfg_c26_limit_swept_through_every_field` | [x] |
| C27 | `read_side_info` | A11=ample + A12 sweep: `main_data_begin` and `part_23_length` chosen so the l.159 check lands on both sides of the boundary and exactly on it | `cfg_c25_c27_main_data_begin_reservoir_boundary` | [x] |
| C28 | `read_side_info` | fully random: random `hdr[0..4]`, random 512-byte bitstream, random `pos`/`limit` — 20 000 iterations, all shapes mixed | `cfg_c28_full_random` | [x] |
| C29 | `read_side_info` | exhaustive header sweep: all 256 `hdr[1]` × all 4 `(hdr[2]>>2)&3` × all 4 `(hdr[3]&0xC0)` values (covers every reachable A1/A2/A4 triple, including the invalid sample-rate index 3 and reserved MPEG version bits) | `cfg_c29_exhaustive_header_sweep` | [x] |
| C30 | `read_side_info` | `region_count[2]` **retention**: A5=1 never writes `region_count[2]`, so the caller's pre-existing byte must survive identically; driven with several distinct pre-fill patterns | `cfg_c30_region_count2_retention` | [x] |
| C31 | `read_side_info` | `sr_idx` decrement quirk: raw sum `0` and raw sum `1` both collapse to `sr_idx = 0`, driven from two different `hdr` encodings | `cfg_c31_sr_idx_decrement_quirk` | [x] |
| C32 | `read_side_info` | `tables` width difference: A5=1 reads only 10 bits then `<<= 5` (so `table_select[2]` is always `0`), A5=0 reads 15 bits (all three `table_select` significant) | `cfg_c32_tables_width_difference` | [x] |
| C33 | `read_side_info` | `hdr` **aliasing the output array**: the C re-reads `hdr[1]`/`hdr[3]` from inside the granule loop, interleaved with its writes through `gr`, so the *timing* of those loads is observable. Driven at every scalar-field byte offset of granules 0 and 1. | `bnd_b12_hdr_aliasing_gr_array` | [x] |

### Note on the C build configuration

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference build is
gcc with **no `-O` flag**. That matters for two rows:

* **C15–C17** — the relative order of the three tables in `.rodata` is
  optimisation-dependent. Measured with gcc 11.5:

  | flags | layout |
  |-------|--------|
  | none / `-O0` | `g_scf_long` @0, 8 pad bytes, `g_scf_short` @192, `g_scf_mixed` @512 |
  | `-O1` / `-O2` / `-O3` / `-Os` | `g_scf_mixed` @0, `g_scf_short` @320, `g_scf_long` @640 (no padding) |

  `src/lib.rs` reproduces the **unoptimised** layout, i.e. the one the documented
  cmake invocation actually produces, and `sym_layout_matches_c_rodata` compares
  the Rust blob against the C `.so`'s `.rodata` at test time so this cannot drift
  unnoticed. (Rows `0..=7` — every in-bounds access — are byte-identical in both
  layouts; only the one-past-the-end `sr_idx == 8` aliasing differs.)

* **C33** — with no `-O` flag gcc reloads `hdr[1]`/`hdr[3]` at every access, so
  the translation must not hoist them.

### Note on C17 (`g_scf_mixed[8]`)

`g_scf_mixed` is the **last** of the three arrays in `.rodata`, so
`g_scf_mixed[8]` reads past the end of the section into linker-generated
`.eh_frame_hdr` bytes. Those bytes are not library data and are not
reproducible in any Rust object. C17 therefore asserts parity of the return
value, `bs->pos` and **all 16 scalar struct fields**, and asserts that the
`sfbtab` pointer lands at the same `(table, row)` offset, but does not compare
the pointed-to bytes. Rows C15 and C16 alias *within* the three tables and are
byte-compared in full.
