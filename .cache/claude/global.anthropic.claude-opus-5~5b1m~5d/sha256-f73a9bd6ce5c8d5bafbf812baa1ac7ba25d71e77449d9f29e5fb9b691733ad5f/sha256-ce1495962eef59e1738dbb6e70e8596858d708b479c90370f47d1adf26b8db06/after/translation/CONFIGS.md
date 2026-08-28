# CONFIGS.md — configuration surface table (Phase B)

Axes derived mechanically from `c_src/src/lib.c` (there is no options struct;
the "options" are the function parameters the code branches on).

**Runtime option axes**

| axis | values the C code distinguishes | where it branches |
|------|--------------------------------|-------------------|
| `mode` (`STBDS_HM_*`) of `hmget/hmput/hmdel_key` | `mode >= STBDS_HM_STRING(1)` ⇒ string hashing/compare; else binary | `stbds_hm_find_slot`, `stbds_hmput_key`, `stbds_is_key_equal`, `stbds_hmdel_key` |
| `table->string.mode` (`STBDS_SH_*`) | `NONE(0)` ⇒ `memcpy` key; `DEFAULT(1)` ⇒ store caller pointer; `STRDUP(2)` ⇒ `stbds_strdup`; `ARENA(3)` ⇒ `stbds_stralloc` | `switch` in `stbds_hmput_key`; strdup sweep in `hmfree_func`; free in `hmdel_key` |
| how the map is created | `stbds_shmode_func(elemsize, mode)` (table pre-made, explicit `string.mode`) vs implicit creation inside `hmput_key`/`hmget_key_ts`/`hmput_default` (`string.mode = SH_DEFAULT` iff `mode >= STBDS_HM_STRING`, else 0) | `stbds_hmput_key` `table == NULL` branch |
| global hash seed | `stbds_rand_seed(s)`; every `make_hash_index(.., NULL)` consumes and advances it (`seed = seed*a + b`); rehash/shrink **inherits** the old seed | `stbds_make_hash_index` |
| `keyoffset` | `0` (all put/get paths hard-code it) vs caller-supplied in `hmdel_key` | `stbds_is_key_equal`, `hmdel_key` re-find |

**Input-shape axes**: `elemsize` (1,3,4,8,12,16,64), `keysize`
(0,1,2,3,4,5,8,16, `< elemsize` and `== elemsize`), element count
(0,1,5,**6**,7,20,100,1000 — 6 is `used_count_threshold` for `slot_count=8`),
key byte patterns (zeros, high-bit ≥0x80, duplicates, collisions), string
lengths (0,1,7,8,9,511,512,513,1<<20+1), delete position (last vs interior),
delete volume (none / tombstone rebuild / shrink), `addlen`/`min_cap`
(0,1,2,4,5,7,100,1<<20), `n` for `strkey` (0, ±, `INT_MIN`, `INT_MAX`),
`num` for `hm_geti` (≤0 … 1000).

Every row is driven with **many randomized inputs** from a fixed-seed
SplitMix64 PRNG, and after *every* individual library call both sides are
compared with a full structural snapshot (array header `length`/`capacity`/
`temp`, all `length*elemsize` element bytes — with `char*` keys compared by
pointee string — plus the whole `stbds_hash_index`: `slot_count`,
`used_count`, all three thresholds, `tombstone_count`, `seed`,
`slot_count_log2`, `string.{remaining,block,mode}`, and every
`hash[8]`/`index[8]` bucket).  Pointer *identity* relations
(`ret == arg`, `ret == NULL`) are compared too.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, any seed (pointer never read) | `phase_b_hash::hash_bytes_len0` | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (tail `switch` fall-through, every `case`), random bytes incl. `>= 0x80` | `phase_b_hash::hash_bytes_tail` | [x] |
| 3 | `stbds_hash_bytes` | `len = 8..64` multiples of 8 (main loop only, `rem == 0`) | `phase_b_hash::hash_bytes_words` | [x] |
| 4 | `stbds_hash_bytes` | `len = 9..255` non-multiples (main loop **and** tail), random | `phase_b_hash::hash_bytes_mixed` | [x] |
| 5 | `stbds_hash_bytes` | high-bit bytes at index 3 and 7 of each word (the sign-extension quirk) | `phase_b_hash::hash_bytes_signext` | [x] |
| 6 | `stbds_hash_bytes` | seed ∈ {0, 1, 0x31415926, SIZE_MAX, random×64} × len ∈ {0..40} | `phase_b_hash::hash_bytes_seeds` | [x] |
| 7 | `stbds_hash_string` | `""`, 1 char, 2..64 chars, bytes `0x01..0xff` incl. `>= 0x80` (unsigned-char promotion) | `phase_b_hash::hash_string_shapes` | [x] |
| 8 | `stbds_hash_string` | seed ∈ {0, 1, 0x31415926, SIZE_MAX, random×64} | `phase_b_hash::hash_string_seeds` | [x] |
| 9 | `stbds_rand_seed` + map creation | seed ∈ {0,1,default,SIZE_MAX,random}; observe the seed the next table gets and the `seed*a+b` advance over N creations | `phase_b_hash::rand_seed_sequence` | [x] |
| 10 | `stbds_arrgrowf` | `a = NULL` × `elemsize` ∈ {1,4,8,12,16,64} × `addlen` ∈ {0,1,2,7,100} × `min_cap` ∈ {0,1,3,4,5,1000} (covers `min_len>min_cap`, `<4` clamp) | `phase_b_arr::growf_fresh_matrix` | [x] |
| 11 | `stbds_arrgrowf` | existing array, request that fits (`min_cap <= cap`) ⇒ no-op identical pointer | `phase_b_arr::growf_noop` | [x] |
| 12 | `stbds_arrgrowf` | existing array, doubling path (`min_cap < 2*cap`) vs explicit-bigger path (`min_cap >= 2*cap`), randomized `addlen`/`min_cap` sequences | `phase_b_arr::growf_repeated` | [x] |
| 13 | `stbds_arrgrowf` | `elemsize = 1`, `min_cap = 1<<20` (large allocation, wrapping arithmetic) | `phase_b_arr::growf_large` | [x] |
| 14 | `stbds_arrfreef` | live array from `arrgrowf` (frees `header`) — run under repeated alloc/free to catch double-free | `phase_b_arr::arrfreef_roundtrip` | [x] |
| 15 | `stbds_hmput_default` | `a = NULL`; then again on the returned map (`length != 0` no-op); then on a map from `hmget_key_ts(NULL)`; `elemsize` ∈ {8,16,4} | `phase_b_map::put_default_paths` | [x] |
| 16 | `stbds_hmget_key` / `_ts` | map with **no hash table** (only `hmput_default`/`hmget_key(NULL)`) ⇒ `temp = -1` | `phase_b_map::get_without_table` | [x] |
| 17 | binary map: `hmput_key`+`hmget_key` | `mode=0`, `elemsize=8`, `keysize=4` (int→int), counts 0,1,5,6,7,20 (crossing `used_count_threshold=6` ⇒ grow 8→16) | `phase_b_map::binary_int_counts` | [x] |
| 18 | binary map | `mode=0`, `elemsize=8/keysize=4`, 100 and 1000 random keys (multiple grows: 8→16→32…) | `phase_b_map::binary_int_many` | [x] |
| 19 | binary map | `mode=0`, `elemsize=16`, `keysize=8` (64-bit keys, payload 8) | `phase_b_map::binary_e16_k8` | [x] |
| 20 | binary map | `mode=0`, `elemsize=16`, `keysize=4` (payload 12, keysize < elemsize) | `phase_b_map::binary_e16_k4` | [x] |
| 21 | binary map | odd shapes: `elemsize=1/keysize=1`, `elemsize=3/keysize=3`, `elemsize=5/keysize=2`, `elemsize=64/keysize=16` | `phase_b_map::binary_odd_shapes` | [x] |
| 22 | binary map | duplicate keys: re-`hmput_key` an existing key (update path, `temp` = existing index, no `used_count` change) | `phase_b_map::binary_duplicates` | [x] |
| 23 | binary map | delete the **last** element (`old_index == final_index`, no memmove) | `phase_b_map::binary_del_last` | [x] |
| 24 | binary map | delete an **interior** element (`old_index != final_index` ⇒ memmove + re-find + re-index) | `phase_b_map::binary_del_interior` | [x] |
| 25 | binary map | delete enough to pass `tombstone_count > tombstone_count_threshold` ⇒ rebuild at same `slot_count` | `phase_b_map::binary_tombstone_rebuild` | [x] |
| 26 | binary map | delete enough to pass `used_count < used_count_shrink_threshold && slot_count > 8` ⇒ shrink (`slot_count>>1`) | `phase_b_map::binary_shrink` | [x] |
| 27 | binary map | re-insert after deletes ⇒ tombstone slot reuse (`tombstone >= 0` branch in `hmput_key`) | `phase_b_map::binary_tombstone_reuse` | [x] |
| 28 | binary map | randomized interleaved scripts of put/get/get_ts/del/put_default/free, 40 seeds × 400 ops, key domain 24 (forces duplicates, misses, empty map) | `phase_b_map::binary_random_scripts` | [x] |
| 29 | binary map | `keysize = 0` (degenerate: every key hashes/compares equal) | `phase_b_map::binary_keysize0` | [x] |
| 30 | `hmfree_func` then reuse | free a populated map, re-insert into the NULL pointer, free again (`hm_geti`'s tail pattern) | `phase_b_map::free_and_reuse` | [x] |
| 31 | string map, implicit creation | `hmput_key(NULL, .., mode=1)` ⇒ `string.mode = SH_DEFAULT`; caller-owned key pointers; puts/gets/dels, counts 1,5,6,7,20 | `phase_b_strmap::implicit_default` | [x] |
| 32 | string map via `shmode_func` | `STBDS_SH_DEFAULT(1)` × `mode=1`, random keys, duplicates, misses, deletes | `phase_b_strmap::sh_default` | [x] |
| 33 | string map via `shmode_func` | `STBDS_SH_STRDUP(2)` × `mode=1`: keys duplicated on insert, freed on delete and in `hmfree_func` | `phase_b_strmap::sh_strdup` | [x] |
| 34 | string map via `shmode_func` | `STBDS_SH_ARENA(3)` × `mode=1`: keys copied into the arena; enough/long keys to force several arena blocks (`block` 0→n) | `phase_b_strmap::sh_arena` | [x] |
| 35 | string map via `shmode_func` | `STBDS_SH_NONE(0)` × `mode=1`: `switch` `default` ⇒ raw `memcpy` of the 8 pointer bytes, then `strcmp` through it | `phase_b_strmap::sh_none_string` | [x] |
| 36 | string map | key shapes: `""`, 1 char, shared prefixes, 511/512/513-byte keys, bytes `>= 0x80`, duplicate keys | `phase_b_strmap::key_shapes` | [x] |
| 37 | string map | `mode = 2..7` and `INT_MAX` (out-of-enum but `>= STBDS_HM_STRING`) behave exactly like `mode = 1` | `phase_b_strmap::mode_out_of_range_valid` | [x] |
| 38 | string map | interior delete with re-find through `*(char**)` (`mode==STRING` re-find branch) + shrink/rebuild | `phase_b_strmap::string_del_paths` | [x] |
| 39 | cross-mode map | table made by `shmode_func(SH_STRDUP)` but used with `mode = 0` (binary hash/compare **and** `strdup` on insert) | `phase_b_strmap::cross_strdup_binary` | [x] |
| 40 | cross-mode map | table made by `shmode_func(SH_ARENA)` but used with `mode = 0` (binary compare, arena copy) | `phase_b_strmap::cross_arena_binary` | [x] |
| 41 | `stbds_stralloc` | fresh zeroed arena (`block=0`, `remaining=0`): first alloc takes the `512 << 0` block | `phase_b_arena::stralloc_fresh` | [x] |
| 42 | `stbds_stralloc` | fill one block exactly then overflow it (`len > remaining`) ⇒ new block, `block` increments, blocksize doubles every other step | `phase_b_arena::stralloc_block_growth` | [x] |
| 43 | `stbds_stralloc` | `len > blocksize` with `storage == NULL` (huge first string) and with `storage != NULL` (splice after head, `remaining` kept) | `phase_b_arena::stralloc_oversized` | [x] |
| 44 | `stbds_stralloc` | `block` pre-set to 0..127 (blocksize clamp at `1<<20`, `block` stops advancing at 22) | `phase_b_arena::stralloc_block_field` | [x] |
| 45 | `stbds_stralloc` | randomized string lengths 0..2000, 200 allocations, contents verified byte-for-byte | `phase_b_arena::stralloc_random` | [x] |
| 46 | `stbds_strreset` | empty arena, 1-block arena, many-block arena, oversized-block arena; arena reused after reset | `phase_b_arena::strreset_paths` | [x] |
| 47 | `strkey` | `n` ∈ {0,1,-1,9,10,42,99999,INT_MAX,INT_MIN} + 200 random; full 256-byte static buffer compared (checks residue from previous longer calls) | `phase_b_driver::strkey_values` | [x] |
| 48 | `hm_geti` | `num` ∈ {0,1,2,3,4,5,6,7,8,9,12,16,17,24,32,50,100,400,1000} — internal grow/delete/shrink/rebuild paths; seed advance compared afterwards | `phase_b_driver::hm_geti_counts` | [x] |
| 49 | `hm_geti` | `num` ≤ 0: {0,-1,-100,INT_MIN} (all loops skipped) | `phase_b_driver::hm_geti_nonpositive` | [x] |
| 50 | `hm_geti` | after `stbds_rand_seed(s)` for s ∈ {0,1,default,SIZE_MAX,random×8} (different table seeds ⇒ different probe orders) | `phase_b_driver::hm_geti_seeds` | [x] |
| 51 | full pipeline | `rand_seed` → `shmode_func` → N×`hmput_key` → `hmget_key`/`_ts` → `hmdel_key` → `hmput_key` again → `hmfree_func`, randomized over all 4 `SH_*` modes × both `HM_*` modes × 3 `elemsize` (the composed pipeline, 24 combos × 20 seeds) | `phase_b_pipeline::full_matrix` | [x] |
| 52 | `hmput_key` (update path) | `SH_DEFAULT`/`SH_STRDUP`/`SH_ARENA` × `HM_STRING`/`HM_BINARY`, 1-5 keys (kept below `used_count_threshold` so the table is never rebuilt), duplicate puts in both directions × 36 seeds — verifies `stbds_temp_key` is written when the existing key is found in the first probe loop and left **stale** when it is found in the wrap-around loop | `phase_b_tempkey::temp_key_on_update`, `phase_b_tempkey::temp_key_binary_mode` | [x] |
| 53 | every entry point (heap parity) | the same create/fill/delete/free workload run 400× per library over all 4 `SH_*` × `HM_*` modes, key lengths 8/300, plus pure `stralloc`/`strreset` and binary-map workloads — the per-iteration `mallinfo2().uordblks` slope must match, including the configurations where the C leaks on purpose (`hmdel_key` frees the duplicated key only when `mode == STBDS_HM_STRING` *exactly*, so `mode = 2/7` leaks ~12800 B/iteration in **both** libraries) | `phase_d_heap_parity::heap_accounting_parity` | [x] |

## Notes on configurations that are deliberately *not* randomised

* `SH_NONE` + `mode >= STBDS_HM_STRING` (row 35): the `switch` `default` arm
  `memcpy`s `keysize` bytes **of the string** into the element, so the stored
  "key pointer" is really string bytes.  Any `stbds_is_key_equal` call in that
  state dereferences them — the C crashes exactly like the Rust would, so the
  scripts only insert distinct keys and look up absent ones.
* `mode >= 2` + interior delete (row 37): `stbds_hmdel_key` takes its
  `mode == STBDS_HM_STRING` false branch and re-hashes the *address bytes* of
  the moved element, which is genuinely address-dependent in the C.  Only
  last-element deletes (`old_index == final_index`) are exercised there.
* `stbds_string_arena.block >= 25` (row 44): `512 << (block>>1)` asks malloc for
  gigabytes; `block >= 128` shifts a `size_t` by ≥ 64 (UB in C).  Rows stop at
  `block = 24`, which already covers the `1<<20` clamp at `block = 22`.
