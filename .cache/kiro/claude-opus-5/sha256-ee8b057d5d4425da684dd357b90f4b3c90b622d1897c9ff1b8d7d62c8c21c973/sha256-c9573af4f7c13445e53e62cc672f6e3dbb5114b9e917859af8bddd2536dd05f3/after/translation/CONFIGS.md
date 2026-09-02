# CONFIGS.md — configuration surface table (valid inputs)

Mechanically enumerated from the branches `c_src/src/lib.c` actually takes.
There is no build-time configuration (no `#ifdef` the CMake build can flip:
`STBDS_SIPHASH_2_4` is hard-wired, `STBDS_REALLOC`/`STBDS_FREE` are fixed to
`realloc`/`free`, and `Cargo.toml` declares **no `[features]`**, so the only
feature combination is the default one).  All configuration is therefore
*runtime* configuration, on these axes:

| axis | values the C branches on | where |
|------|--------------------------|-------|
| A. `mode` (hash-map key mode) | `0` = `STBDS_HM_BINARY`; `1` = `STBDS_HM_STRING`; `>=2` (PTR_TO_STRING-like, hits `>=` but not `==`); negative | `mode >= STBDS_HM_STRING` (L560,590,713,732), `mode == STBDS_HM_STRING` (L836,842) |
| B. `string.mode` (arena/key-storage mode, set by `stbds_shmode_func`) | `SH_NONE=0`, `SH_DEFAULT=1`, `SH_STRDUP=2`, `SH_ARENA=3`, out-of-range | `switch (table->string.mode)` (L787), L575, L836 |
| C. `elemsize` | 4, 8, 16, 24, 32 … (must be `>= keysize`; `>= 8` for the pointer-key modes) | every `elemsize*i` |
| D. `keysize` | 0, 1, 2, 3, 4, 8, 16 (and, for string modes, `sizeof(char*)`) | `memcmp(...,keysize)`, `hash_bytes(key,keysize,..)` |
| E. `keyoffset` | 0 (all internal callers) and non-zero (only `hmdel_key` takes it) | `elemsize*i + keyoffset` |
| F. element count | 0, 1, 2, 6 (= `used_count_threshold` for 8 slots → first rehash), 7, 8, 64, 200 (multiple rehashes) | L698, L858, L861 |
| G. table `slot_count` | 8 (initial, never shrinks), 16, 32, 64 … after growth; `>>1` after shrink | L702, L858, L861, L398 |
| H. `arrgrowf` shape | `a==NULL` vs non-null; `addlen` 0/1/n; `min_cap` 0/1/3/4/large; `min_cap<=cap` no-op; `min_cap < 2*cap` doubling | L283-L291 |
| I. `hash_bytes` length | 0,1,2,3,4,5,6,7 (each tail `switch` case), 8, 9, 15, 16, 17, 31, 32, 64, 129 | L516-L537 |
| J. `hash_bytes` byte values | all-zero, all-`0xff`, byte 3 / byte 7 high-bit set (the `int` sign-extension quirk), random | L520-L521, L532 |
| K. `hash_string` content | `""`, 1 char, 7/8/9 chars, ASCII, bytes `>= 0x80`, long (>512) | L466-L469 |
| L. seed | default `0x31415926`, `0`, `1`, `SIZE_MAX`, random (via `stbds_rand_seed`) | L376, L406-L410 |
| M. arena state (`stbds_stralloc`) | fresh zeroed arena; `block` 0/1/2/…/22 (blocksize progression `512<<(block>>1)`); `remaining` 0 vs partial; string shorter vs longer than `blocksize` | L885-L911 |
| N. delete position | first, middle, last (`old_index == final_index` → skip compaction), only element | L840 |
| O. key distribution | distinct, duplicate (re-put existing key), re-insert after delete (tombstone reuse) | L727-L760 |

Rows below are the pruned cross-product — one row per combination the C treats
differently.  Every row is exercised with **many randomized inputs** (fixed
seed, see `tests/common/mod.rs`), driving the **lowest-level** exports
directly (`stbds_arrgrowf`, `stbds_hmput_key`, `stbds_hmget_key_ts`,
`stbds_hmdel_key`, `stbds_shmode_func`, `stbds_stralloc`) rather than only the
convenience entry points.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | len = 0, random bytes, random seeds | [x] |
| 2 | `stbds_hash_bytes` | len = 1..7 (every tail `switch` case), random bytes, random seeds | [x] |
| 3 | `stbds_hash_bytes` | len = 8 exactly (one full block, empty tail) | [x] |
| 4 | `stbds_hash_bytes` | len = 9..15 (one block + each tail case) | [x] |
| 5 | `stbds_hash_bytes` | len = 16, 24, 32, 64 (multi-block, no tail) | [x] |
| 6 | `stbds_hash_bytes` | len = 17, 31, 33, 129 (multi-block + tail) | [x] |
| 7 | `stbds_hash_bytes` | all-zero buffer, every len 0..64 | [x] |
| 8 | `stbds_hash_bytes` | all-`0xff` buffer, every len 0..64 | [x] |
| 9 | `stbds_hash_bytes` | byte 3 (and byte 7) forced `>= 0x80` — `int` sign-extension path, every len 4..64 | [x] |
|10 | `stbds_hash_bytes` | seed = 0, 1, `usize::MAX`, `0x31415926`, random; fixed buffer | [x] |
|11 | `stbds_hash_string` | `""`, seed random | [x] |
|12 | `stbds_hash_string` | 1..40 ASCII chars, random content and seeds | [x] |
|13 | `stbds_hash_string` | bytes `0x80..0xff` (high-bit, `unsigned char` promotion), random lens | [x] |
|14 | `stbds_hash_string` | long string (600 chars, > arena blocksize) | [x] |
|15 | `stbds_rand_seed` + `stbds_hash_bytes`/`hash_string` | seeding does *not* affect these (seed is an argument) — confirm both ignore the global | [x] |
|16 | `stbds_arrgrowf` | `a == NULL`, `addlen = 0`, `min_cap` ∈ {0,1,2,3,4,5,17,1000}, `elemsize` ∈ {1,4,8,16,24} | [x] |
|17 | `stbds_arrgrowf` | `a == NULL`, `addlen` ∈ {1,4,7,64}, `min_cap = 0` | [x] |
|18 | `stbds_arrgrowf` | non-null `a`, `min_cap <= cap`, `addlen = 0` → no-op, pointer identity preserved | [x] |
|19 | `stbds_arrgrowf` | non-null `a`, `min_cap` between `cap+1` and `2*cap` → doubling path | [x] |
|20 | `stbds_arrgrowf` | non-null `a`, `min_cap > 2*cap` → exact `min_cap` path | [x] |
|21 | `stbds_arrgrowf` | non-null `a`, growth driven by `addlen` (`min_len > min_cap`) | [x] |
|22 | `stbds_arrgrowf` | repeated randomized grow chain (100 steps) with random `addlen`/`min_cap`, checking `length`/`capacity`/`temp`/payload after each step | [x] |
|23 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free, non-null | [x] |
|24 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=8`, `keysize=4`, 1 key | [x] |
|25 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=8`, `keysize=4`, N ∈ {2,5,6,7,8} keys — crosses the 8-slot `used_count_threshold=6` rehash | [x] |
|26 | `stbds_hmput_key` (binary) | `mode=0`, N = 64 and 200 random distinct keys — multiple rehashes (8→16→32→…) | [x] |
|27 | `stbds_hmput_key` (binary) | duplicate keys re-put (updates `temp`, does not grow `length`) | [x] |
|28 | `stbds_hmput_key` (binary) | `keysize` ∈ {1,2,3,4,8,16} × `elemsize` ∈ {keysize, keysize+4, 32} | [x] |
|29 | `stbds_hmput_key` (binary) | `keysize = 0` (every key compares equal) | [x] |
|30 | `stbds_hmput_key` (binary) | keys crafted so `hash < 2` cannot be forced directly; instead random seeds via `stbds_rand_seed` to shuffle the probe order | [x] |
|31 | `stbds_hmget_key_ts` | lookup of a present key, binary mode, table of each size class | [x] |
|32 | `stbds_hmget_key_ts` | lookup of an absent key → `*temp = -1` | [x] |
|33 | `stbds_hmget_key_ts` | `a == NULL` bootstrap | [x] |
|34 | `stbds_hmget_key_ts` | `a` from `arrgrowf` with `hash_table == NULL` | [x] |
|35 | `stbds_hmget_key` | same as 31–34, plus `header->temp` side-effect | [x] |
|36 | `stbds_hmput_default` | `a == NULL` | [x] |
|37 | `stbds_hmput_default` | `a` non-null with `length == 0` | [x] |
|38 | `stbds_hmput_default` | `a` non-null with `length != 0` → no-op | [x] |
|39 | `stbds_hmput_default` → `stbds_hmput_key` | default set first, then puts (binary) | [x] |
|40 | `stbds_hmdel_key` (binary) | delete present key, `old_index == final_index` (last element) | [x] |
|41 | `stbds_hmdel_key` (binary) | delete present key, `old_index != final_index` (compaction + re-find) | [x] |
|42 | `stbds_hmdel_key` (binary) | delete absent key → `temp = 0`, unchanged | [x] |
|43 | `stbds_hmdel_key` (binary) | delete until `used_count < used_count_shrink_threshold` on a 16+-slot table → shrink rebuild | [x] |
|44 | `stbds_hmdel_key` (binary) | delete/insert churn until `tombstone_count > tombstone_count_threshold` → same-size rebuild | [x] |
|45 | `stbds_hmdel_key` (binary) | delete then re-insert the same key → tombstone reuse in `hmput_key` | [x] |
|46 | `stbds_hmdel_key` (binary) | non-zero `keyoffset` with `elemsize` big enough to hold `{pad, key}` | [x] |
|47 | `stbds_hmdel_key` (binary) | randomized put/get/del sequence, 400 ops, `elemsize=16`, `keysize=4` | [x] |
|48 | `stbds_shmode_func` | `mode = SH_NONE(0)`, `elemsize` ∈ {8,16,24} | [x] |
|49 | `stbds_shmode_func` | `mode = SH_DEFAULT(1)` | [x] |
|50 | `stbds_shmode_func` | `mode = SH_STRDUP(2)` | [x] |
|51 | `stbds_shmode_func` | `mode = SH_ARENA(3)` | [x] |
|52 | `stbds_shmode_func` + `stbds_hmput_key` | `string.mode = SH_NONE`, `mode = STBDS_HM_STRING(1)` → `default:` `memcpy` branch even though `mode` is "string" | [x] |
|53 | `stbds_shmode_func` + `stbds_hmput_key` | `string.mode = SH_DEFAULT`, `mode = 1`, N random C strings — key pointer stored verbatim | [x] |
|54 | `stbds_shmode_func` + `stbds_hmput_key` | `string.mode = SH_STRDUP`, `mode = 1`, N random C strings — key `strdup`'d | [x] |
|55 | `stbds_shmode_func` + `stbds_hmput_key` | `string.mode = SH_ARENA`, `mode = 1`, N random C strings — key arena-allocated | [x] |
|56 | `stbds_shmode_func` + `stbds_hmput_key` | `string.mode = SH_ARENA`, keys long enough to blow past `blocksize` (mixed short/long, > 512 bytes) | [x] |
|57 | `hmput_key`/`hmget_key`/`hmdel_key` string mode | `SH_DEFAULT`, N ∈ {1,6,7,8,64} keys — crosses rehash with string keys | [x] |
|58 | `hmput_key`/`hmget_key`/`hmdel_key` string mode | `SH_STRDUP`, delete present key (`mode==1` → frees the dup) | [x] |
|59 | `hmput_key`/`hmget_key`/`hmdel_key` string mode | `SH_ARENA`, delete present key (`mode==1`, no free) | [x] |
|60 | `hmput_key`/`hmdel_key` string mode | duplicate string key re-put — hits the `temp_key` update in the forward half-scan | [x] |
|61 | `hmput_key`/`hmget_key` | `mode = 2` (out-of-range-but-`>=STBDS_HM_STRING`) with `SH_DEFAULT` storage | [x] |
|62 | `hmdel_key` | `mode = 2` with `SH_STRDUP` storage → the `mode == STBDS_HM_STRING` guard is false, so no free and the *undereferenced* element address is used on re-find | [x] |
|63 | `hmput_key`/`hmget_key`/`hmdel_key` | `mode = -1` and `mode = i32::MIN` → binary path | [x] |
|64 | `hmput_key`/`hmget_key`/`hmdel_key` | `mode = 1000` / `i32::MAX` → string path | [x] |
|65 | `stbds_stralloc` | fresh zeroed arena, string lens 1..40, sequential allocations | [x] |
|66 | `stbds_stralloc` | fresh arena, first string longer than 512 (`len > blocksize`, empty-arena splice) | [x] |
|67 | `stbds_stralloc` | non-empty arena, then a string longer than the current `blocksize` (non-empty splice: `sb->next = storage->next`) | [x] |
|68 | `stbds_stralloc` | drive `a->block` 0→…→8 by exhausting blocks (blocksize progression `512,512,1024,1024,…`) | [x] |
|69 | `stbds_stralloc` | pre-seeded arena with `block` ∈ {0,1,2,5,10,20,22} and `remaining = 0` | [x] |
|70 | `stbds_stralloc` | `""` (len 1) repeatedly, 600 times — exhausts a 512-byte block and rolls over | [x] |
|71 | `stbds_strreset` | zeroed arena (no-op) | [x] |
|72 | `stbds_strreset` | arena with 1 block, and with many blocks | [x] |
|73 | `stbds_hmfree_func` | `p == NULL`; `hash_table == NULL`; `SH_NONE`; `SH_DEFAULT`; `SH_STRDUP` (frees each key); `SH_ARENA` (resets the arena) | [x] |
|74 | `strkey` | `n` ∈ {0,1,-1,7,42,i32::MAX,i32::MIN} + 200 random `i32` | [x] |
|75 | `arr_del` | `num` ∈ {0,1,-1,i32::MAX,i32::MIN} + 200 random `i32` (must not abort; exercises `arrdel`+`arrdelswap` at i=0..3) | [x] |
|76 | `stbds_rand_seed` + `hmput_key` | reseed to {0,1,`usize::MAX`,random} then build a 64-key binary table — table `seed` comes from the global and the global advances via the LCG; verifies the LCG and the seed-inheritance-on-rehash path | [x] |
|77 | end-to-end pipeline | reseed → `shmode_func(SH_STRDUP)` → 100 randomized puts → gets → 50 deletes → gets → `hmfree_func`, comparing `length`/`temp`/every key string after every step | [x] |
|78 | end-to-end pipeline | reseed → binary map, 300-op randomized put/get/del mix, `elemsize=24`, `keysize=8`, comparing full array payload after every op | [x] |
|79 | `stbds_hmput_key` from `NULL` with `mode >= STBDS_HM_STRING` | no `shmode_func` involved — the function builds its own 8-slot index and sets `string.mode = SH_DEFAULT` itself (the `shput`-on-a-fresh-map path), `mode` ∈ {1,2,1000,`i32::MAX`}, `elemsize` ∈ {8,16,24} | [x] |

## Row to test mapping

| rows | test file :: test |
|------|-------------------|
| 1–10 | `b_hash::row_1_6_hash_bytes_all_lengths`, `row_7_hash_bytes_all_zero`, `row_8_hash_bytes_all_ff`, `row_9_hash_bytes_sign_extension`, `row_10_hash_bytes_seed_sweep` |
| 11–14 | `b_hash::row_11_12_hash_string_ascii`, `row_13_hash_string_high_bit`, `row_14_hash_string_long` |
| 15 | `b_hash::row_15_rand_seed_does_not_affect_pure_hashes` |
| 16–23 | `b_arr::row_16_grow_from_null_min_cap` … `row_23_grow_then_free`, `err_row_2_addlen_wraparound` |
| 24–30 | `b_hmap_binary::row_24_26_put_counts`, `row_27_duplicate_puts`, `row_28_keysize_elemsize_matrix`, `row_29_keysize_zero`, `row_30_76_global_seed_variation` |
| 31–35 | `b_hmap_binary::row_31_32_get_ts_present_absent`, `row_33_35_get_bootstrap_paths` |
| 36–39 | `b_hmap_binary::row_36_39_hmput_default` |
| 40–47 | `b_hmap_binary::row_40_42_del_basics`, `row_43_shrink_rebuild`, `row_44_45_tombstone_rebuild_and_reuse`, `row_46_del_keyoffset`, `row_47_78_randomized_mix` |
| 48–52 | `b_hmap_string::row_48_51_shmode_func`, `err_row_50_shmode_out_of_range`, `row_52_sh_none_string_mode` |
| 53–57 | `b_hmap_string::row_53_57_sh_default`, `row_54_sh_strdup`, `row_55_sh_arena`, `row_56_sh_arena_long_keys` |
| 58–60 | `b_hmap_string::row_58_59_string_deletes`, and the duplicate-re-put phase of `string_roundtrip` |
| 61–64 | `b_hmap_string::row_61_mode_two_default`, `row_62_mode_two_strdup_delete_last`, `row_63_negative_mode`, `row_64_huge_mode_string_path`, `c_errors::err_rows_51_52_out_of_range_mode_enum` |
| 65–72 | `b_arena::row_65_stralloc_fresh_short` … `row_71_72_strreset` |
| 73 | `b_arena::row_73_hmfree_func_shapes` |
| 74–75 | `b_arena::row_74_strkey`, `row_75_arr_del` |
| 76 | `b_hmap_binary::row_30_76_global_seed_variation` |
| 77 | `b_hmap_string::row_77_end_to_end_strdup` |
| 78 | `b_hmap_binary::row_47_78_randomized_mix` |
| 79 | `c_errors::cfg_row_79_hmput_key_null_string_mode` |

## What every row compares

`tests/common/mod.rs` snapshots the *complete* observable state of a map after
every single operation and diffs the C against the Rust:

* array header: `length`, `capacity`, `temp`, `hash_table` non-nullness;
* hash index: `slot_count`, `used_count`, `used_count_threshold`,
  `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, `slot_count_log2`, and the arena
  (`remaining`, `block`, `mode`, storage non-nullness);
* **every slot of every bucket**: all `slot_count` hash values and all
  `slot_count` indices, so probe order, tombstone placement and rehash layout
  are compared bit-for-bit, not just the externally visible answers;
* the element payload — raw bytes for binary keys, the pointed-to strings plus
  the trailing value bytes for the pointer-key modes;
* the returned `*temp` / `header->temp` sentinel of every lookup and delete.

`table->temp_key` is deliberately excluded from the general snapshot because
`stbds_make_hash_index` leaves it uninitialised; it is compared separately (by
pointed-to string) right after each string-mode `hmput_key`, where the C does
define it.

Raw pointer *values* are never compared (the two libraries have independent
allocators), except where the C's contract is pointer identity — the
`arrgrowf` no-op path and the `hmput_default` no-op path, which assert
`returned == input` on both sides.


## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so the complete set of feature combinations is:

| combo | command | status |
|-------|---------|--------|
| default (only combo) | `cargo test --release` | [x] |
| `--no-default-features` (identical to default — no features exist) | `cargo test --release --no-default-features` | [x] |

Verified mechanically by `check_features.sh`.
