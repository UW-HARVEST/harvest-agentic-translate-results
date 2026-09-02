# CONFIGS.md — configuration surface table (valid inputs)

Axes derived from what `c_src/src/lib.c` actually branches on.

## Axes

**A. Entry points (all 16 exported symbols, low-level first)**
`stbds_rand_seed`, `stbds_hash_bytes`, `stbds_hash_string`, `stbds_arrgrowf`,
`stbds_arrfreef`, `stbds_stralloc`, `stbds_strreset`, `stbds_shmode_func`,
`stbds_hmput_default`, `stbds_hmput_key`, `stbds_hmget_key`,
`stbds_hmget_key_ts`, `stbds_hmdel_key`, `stbds_hmfree_func`, `strkey`,
`hm_geti`.

**B. `mode` argument** (`hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key`):
`mode < STBDS_HM_STRING(1)` → binary/`memcmp`/`siphash`;
`mode >= 1` → string/`strcmp`/`hash_string`.
`hmdel_key` additionally tests `mode == 1` *exactly* for strdup-key freeing.

**C. `table->string.mode`** (set by `shmode_func`, or implicitly by
`hmput_key` when it creates the first index): `0` (`SH_NONE`, memcpy),
`1` (`SH_DEFAULT`, store caller pointer), `2` (`SH_STRDUP`, `strdup`),
`3` (`SH_ARENA`, arena-alloc), other u8 → `default:` memcpy.

**D. `elemsize` / `keysize` shapes**: `{key:i32,value:i32}` (8/4),
`{key:*char,value:i32}` (16/8), `{key:[i32;2],b,c,d}` (20/8),
`{key:u8,value:u8}` (2/1), `keysize == 0`, `elemsize == keysize` (no value).

**E. Table population / growth stage**: empty (no index), 1 (default slot only),
`< used_count_threshold` (≤5 live with 8 slots), exactly at the grow boundary
(6 live → rehash to 16), many (several successive doublings), with tombstones
below and above `tombstone_count_threshold`, shrunk (below
`used_count_shrink_threshold` with `slot_count > 8`).

**F. Byte-string shapes for hashing**: `len` 0,1..8,9,15,16,17,31,32,33,63,64,
65, 127, 128 (covers all `len % 8` tail cases and multiple sip loop iterations);
byte values incl. `>= 0x80` (exercises the C sign-extension quirk).

**G. Seed**: default (`0x31415926`), `0`, `1`, `usize::MAX`, random.

**H. Arena block-size stage** (`stralloc`): `remaining` sufficient;
`len > remaining` and `len <= blocksize` (new 512-byte block, then 512, 1024, …
as `block` grows); `len > blocksize` with empty arena; `len > blocksize` with
non-empty arena; `block` at saturation (≥22).

**I. `arrgrowf` growth-decision branches**: no-op; from `NULL`; `min_len >
min_cap`; `min_cap < 2*cap`; `min_cap >= 2*cap && min_cap < 4`;
`min_cap >= max(2*cap, 4)`.

**J. Build configuration**: `Cargo.toml` has no `[features]`, so the only
configurations are the default build and `--no-default-features` (identical).
`verify.sh` enumerates combinations from `Cargo.toml` generically, so adding a
feature later cannot silently skip coverage.

**Findings that changed this table.** Two axes turned out to be *non*-axes in the
ground truth, and the tests now pin that down rather than assuming it:

* `stbds_hash_bytes` ignores its `seed` — `stbds_siphash_bytes` XORs the seed
  into every state word twice, so it cancels. Binary-key bucket layouts are
  therefore identical across seeds (`row37a`, `row37d`).
* `stbds_hash_string` *does* use the seed, so string-key layouts do vary
  (`row37b`, `row37c`).

## Rows (cross-product pruned to combinations the C distinguishes)

Each row is exercised with many randomized inputs (fixed seeds, so every run is
reproducible) through both `.so`s and compared byte-for-byte: return values,
`temp`, header `length`/`capacity`/`temp`, element bytes, and the full hash-index
state (`slot_count`, `used_count`, all three thresholds, `tombstone_count`,
`seed`, `slot_count_log2`, `string.{remaining,block,mode}`, and every bucket's
`hash[8]`/`index[8]`). `[x]` = passing against both `.so` files.

Pointer *values* are never compared where they cannot match (the two libraries
allocate independently); instead the snapshot canonicalises them — string keys
are compared by contents, and arena results by structural position
(`head + 8 + remaining` vs `chain[n] + 8`).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `stbds_hash_bytes` | axis F: `len` ∈ {0,1,…,17,23,24,25,31,32,33,39,63,64,65,127,128,129,255,256} × 64 random buffers each, default seed | `row01_hash_bytes_lengths_default_seed` | [x] |
| 2 | `stbds_hash_bytes` | axis F × axis G: same lengths × 8 seeds × 24 buffers | `row02_hash_bytes_lengths_x_seeds` | [x] |
| 3 | `stbds_hash_bytes` | all-high-bit bytes (`0x80..0xFF`) at every tail length — the `d[3]<<24` / `d[7]<<24` sign-extension paths | `row03_hash_bytes_high_bit_bytes` | [x] |
| 4 | `stbds_hash_string` | empty, 1, 7/8/9, 200 chars; ASCII and bytes `0x80..0xFF`; × 8 seeds | `row04_hash_string` | [x] |
| 5 | `stbds_rand_seed` + `stbds_shmode_func` | 16 seeds × 8 successive fresh tables — per-table `seed` and the global `seed = seed*a + b` advance | `row05_rand_seed_and_table_seed_chain` | [x] |
| 6 | `stbds_arrgrowf` | axis I "no-op": `a=NULL`, `addlen=0`, `min_cap=0` → returns `NULL` | `row06_arrgrowf_noop_from_null` | [x] |
| 7 | `stbds_arrgrowf` | from `NULL`: `elemsize` ∈ {1,2,4,8,16,20,64} × `addlen` ∈ {0,1,2,3,4,5,7,8,63,64,65} × `min_cap` ∈ {0,1,2,3,4,5,8,63,64,65,1024} | `row07_arrgrowf_from_null_grid` | [x] |
| 8 | `stbds_arrgrowf` | growth chains on an existing array: 5 element sizes × 24 trials × 24 randomized steps, payload preserved and compared | `row08_arrgrowf_growth_chain` | [x] |
| 9 | `stbds_arrgrowf` | branch boundaries: `min_cap` at `cap±2`, `2*cap±2`, `4±2`; plus `elemsize == 0` | `row09_arrgrowf_branch_boundaries` | [x] |
| 10 | `stbds_stralloc` + `stbds_strreset` | axis H: 12 trials × 400 random short strings (fits → new block → block growth), then reset; plus empty/exact-fit shapes | `row10_stralloc_short_string_sequence` | [x] |
| 11 | `stbds_stralloc` | `len > blocksize` on an **empty** arena (`remaining` set to 0, head installed) | `row11_stralloc_oversize_on_empty_arena` | [x] |
| 12 | `stbds_stralloc` | `len > blocksize` on a **non-empty** arena (spliced after head, `remaining` preserved) | `row12_stralloc_oversize_on_nonempty_arena` | [x] |
| 13 | `stbds_stralloc` | driven until `block` saturates at 22, then 200 more small strings | `row13_stralloc_block_saturation` | [x] |
| 14 | `stbds_shmode_func` | axis C × axis D: mode ∈ {0,1,2,3} × elemsize ∈ {1,2,4,8,16,20,64} × 4 seeds | `row14_shmode_func_modes_x_elemsizes` | [x] |
| 15 | `stbds_hmput_default` | `a == NULL`; again on the result (`length != 0` no-op); then on a `length == 0` array; × 5 element sizes | `row15_hmput_default` | [x] |
| 16 | `stbds_hmput_key` | binary, `{i32,i32}` (8/4), 96 inserts crossing the 8→16→32→64→128 rehash boundaries, sequential and random keys, ×16 trials | `row16_hmput_binary_int_key_growth` | [x] |
| 17 | `stbds_hmput_key` | binary, elemsize 20 / keysize 8, 120 inserts with ~1/3 duplicate keys | `row17_hmput_binary_wide_key_with_duplicates` | [x] |
| 18 | `stbds_hmput_key` | binary, `keysize == elemsize` (no value field), elemsize ∈ {1,2,4,8,16} | `row18_hmput_binary_key_is_whole_element` | [x] |
| 19 | `stbds_hmput_key` | string mode on the `SH_DEFAULT` table `hmput_key` auto-creates — caller pointers stored, `temp_key` published, duplicates re-put | `row19_shput_sh_default_autocreated` | [x] |
| 20 | `stbds_hmput_key` | string mode on `shmode_func(SH_STRDUP)` — keys duplicated (distinct pointers, equal contents) | `row20_shput_sh_strdup` | [x] |
| 21 | `stbds_hmput_key` | string mode on `shmode_func(SH_ARENA)` — arena `block`/`remaining` tracked across 120 keys incl. long keys forcing dedicated blocks | `row21_shput_sh_arena` | [x] |
| 22 | `stbds_hmput_key` | string mode on `shmode_func(SH_NONE)` → `default:` memcpy branch with a pointer-sized key | `row22_shput_sh_none_default_memcpy_branch` | [x] |
| 23 | `stbds_hmput_key` | `mode` ∈ {2,3,7,99,`INT_MAX`} (out-of-range but "string") on a `SH_DEFAULT` table | `row23_shput_out_of_range_string_modes` | [x] |
| 24 | `stbds_hmget_key` / `_ts` | on `NULL` (bootstrap), on an index-less array, on populated tables: hits and misses over 300 random probes × 8 trials | `row24_hmget_on_null_and_indexless_and_populated` | [x] |
| 25 | `stbds_hmget_key` vs `_ts` | both on the same populated table; `hmget_key` mirrors `temp` into the header, `hmget_key_ts` must not (checked with a sentinel) | `row25_hmget_key_vs_ts_temp_semantics` | [x] |
| 26 | `stbds_hmget_key` | string mode over `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA`: present and absent keys | `row26_shget_across_string_modes` | [x] |
| 27 | `stbds_hmdel_key` | binary: delete-last (`old_index == final_index`) and delete-middle (memmove + slot re-point), plus double-delete | `row27_hmdel_last_and_middle` | [x] |
| 28 | `stbds_hmdel_key` | binary: 200 put/delete churn cycles crossing `tombstone_count_threshold` (same-size rebuild) | `row28_hmdel_tombstone_rebuild` | [x] |
| 29 | `stbds_hmdel_key` | binary: grow to ≥256 slots then drain, crossing every `used_count_shrink_threshold` down to 8 | `row29_hmdel_shrink_rebuild` | [x] |
| 30 | `stbds_hmdel_key` | string mode 1 on `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA`, shuffled deletion order + double-delete | `row30_shdel_across_string_modes` | [x] |
| 31 | `stbds_hmdel_key` | `mode == 2` on a `SH_STRDUP` table: LIFO deletes succeed; a relocating delete aborts in **both** builds (child-compared) | `row31_shdel_mode2_skips_strdup_free` | [x] |
| 32 | put/get/get_ts/del | binary randomized op stream, 2000 ops × 6 trials × 3 key-space sizes, full state compared after **every** op | `row32_random_op_stream_binary` | [x] |
| 33 | put/get/del | string randomized op stream, 1200 ops × 3 trials × each of `SH_DEFAULT`/`SH_STRDUP`/`SH_ARENA` | `row33_random_op_stream_string` | [x] |
| 34 | `stbds_hmfree_func` | on `NULL`, an index-less array, a populated binary table, all four string modes, and a plain `arrgrowf` array | `row34_hmfree_func_shapes` | [x] |
| 35 | `strkey` | `n` ∈ {0,±1,9,10,11,99,100,101,999,1000,±12345,`i32::MIN`,`i32::MAX`,…} + 4000 randomized; plus static-buffer aliasing | `row35_strkey` | [x] |
| 36 | `hm_geti` | `num` ∈ {0,1,…,9,15,16,17,31,33,64,100,257} × 5 seeds, child-compared; plus negatives; plus in-process | `row36_hm_geti_end_to_end` | [x] |
| 37 | seed → layout | `hash_bytes` is seed-invariant (the C cancels the seed); `hash_string` is not; string-map layouts change with the seed, binary-map layouts do not | `row37a`, `row37b`, `row37c`, `row37d` | [x] |
| 38 | build config | no `[features]` in `Cargo.toml`; `DEFAULT` and `NO_DEFAULT` both build, export all 16 symbols, and pass all suites | `./verify.sh all` | [x] |
| S1 | all `hm*` + `hmput_default` | 14 shapes (6 binary widths, 2 negative op-modes, 5 string modes, 1 out-of-range string mode) × 3 trials × 3000 interleaved ops, with free/rebuild cycles; asserts tables reach ≥128 slots | `stress_multi_shape_interleaved` | [x] |
| S2 | put/del | 4 trials × 6 fill/drain cycles of up to 240 keys with shuffled drains — grow and shrink rebuilds many times over | `stress_growth_shrink_cycles` | [x] |
| S3 | `SH_ARENA` + rehash | 400 keys with mixed lengths so the arena embedded in the index is copied across many rehashes and advances its `block` | `stress_arena_inside_hash_index` | [x] |
| S4 | `hash_bytes`/`hash_string` | ~25k random `hash_bytes` over lengths 0..=136 and 20k random `hash_string`, random seeds | `stress_hash_functions_wide` | [x] |
| S5 | `stralloc`/`strreset` | 20 trials × 500 randomized allocations mixing tiny, block-sized and 200 KB strings | `stress_arena_random_sequences` | [x] |
