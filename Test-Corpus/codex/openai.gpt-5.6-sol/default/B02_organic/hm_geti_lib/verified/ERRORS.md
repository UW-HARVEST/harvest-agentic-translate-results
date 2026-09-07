# Error surface

Mechanically derived from every `STBDS_ASSERT`, sentinel return, explicit null
check, range check, and named minimum/maximum in `c_src/src/lib.c`. Internal
invariant assertions are retained because assertion failure is part of the C
observable behavior, even where ordinary API-created state cannot trigger it.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|---|
| 1 | `stbds_arrfreef` | `a == NULL` (generic null boundary; this function has no null guard) | invalid header dereference; process faults | [x] |
| 2 | `stbds_hash_string` | `str == NULL` | process faults while reading `*str` | [x] |
| 3 | `stbds_hash_bytes` | `p == NULL && len == 0` | accepted; returns the zero-length hash for `seed` | [x] |
| 4 | `stbds_hash_bytes` | `p == NULL && len > 0` | process faults while reading input bytes | [x] |
| 5 | `stbds_hmfree_func` | `a == NULL` | returns immediately | [x] |
| 6 | `stbds_hmget_key_ts` | `a == NULL` | creates the one-element default slot, stores `STBDS_INDEX_EMPTY` (`-1`) in `*temp`, returns hash view | [x] |
| 7 | `stbds_hmget_key_ts` | non-null map with `table == NULL` | stores `-1` in `*temp`, returns the same map | [x] |
| 8 | `stbds_hmget_key_ts` / `stbds_hm_find_slot` | key absent and first probe scan reaches `STBDS_HASH_EMPTY` | stores `-1` in `*temp` | [x] |
| 9 | `stbds_hmget_key_ts` / `stbds_hm_find_slot` | wrapped probe scan reaches `STBDS_HASH_EMPTY` | stores `-1` in `*temp` | [x] |
| 10 | `stbds_hmget_key_ts` | `temp == NULL` | process faults when writing the result | [x] |
| 11 | `stbds_hmget_key` | missing key | returns map and stores `-1` in header `temp` | [x] |
| 12 | `stbds_hmput_default` | `a == NULL` | allocates a zeroed default slot and returns hash view | [x] |
| 13 | `stbds_hmput_default` | non-null map whose raw array length is `0` | creates/zeros the default slot | [x] |
| 14 | `stbds_hmdel_key` | `a == NULL` | returns `NULL` | [x] |
| 15 | `stbds_hmdel_key` | map exists but `table == NULL` | returns same map and leaves header `temp == 0` | [x] |
| 16 | `stbds_hmdel_key` | requested key is absent (`slot < 0`) | returns same map and leaves header `temp == 0` | [x] |
| 17 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | `STBDS_ASSERT` abort; generated slot counts must preserve the invariant | [x] |
| 18 | `stbds_hmput_key` | post-growth `i + 1 > capacity` remains true | `STBDS_ASSERT(i + 1 <= capacity)` abort | [x] |
| 19 | `stbds_hmdel_key` | found slot is outside `[0, table->slot_count)` | `STBDS_ASSERT(slot < slot_count)` abort | [x] |
| 20 | `stbds_hmdel_key` | decrement would make `used_count < 0` | `STBDS_ASSERT(table->used_count >= 0)`; with unsigned `size_t`, the C expression remains true after wrap | [x] |
| 21 | `stbds_hmdel_key` | moved final entry cannot be found after compaction | `STBDS_ASSERT(slot >= 0)` abort | [x] |
| 22 | `stbds_hmdel_key` | moved entry's bucket index is not `final_index` | `STBDS_ASSERT(b->index[i] == final_index)` abort | [x] |
| 23 | `stbds_stralloc` | post-allocation `len > a->remaining` on the normal-block path | `STBDS_ASSERT(len <= remaining)` abort | [x] |
| 24 | `stbds_stralloc` | string length crosses `STBDS_STRING_ARENA_BLOCKSIZE_MIN` (`512`) | select normal block when it fits, otherwise a dedicated allocation | [x] |
| 25 | `stbds_stralloc` | arena growth reaches `STBDS_STRING_ARENA_BLOCKSIZE_MAX` (`1<<20`) | stop incrementing `a->block`; future normal blocks remain at the maximum | [x] |
| 26 | `stbds_shmode_func` / map APIs | mode is outside enum values `0..=3` | accepted as an `int`, stored after unsigned-char conversion; later behavior follows the C comparisons/switch default | [x] |
| 27 | `hm_geti` | initial lookup of key `1` returns anything other than `-1` | assertion abort | [x] |
| 28 | `hm_geti` | lookup of key `1` after setting default returns an index other than `-1` | assertion abort | [x] |
| 29 | `hm_geti` | missing key after setting default returns value other than `-2` | assertion abort | [x] |
| 30 | `hm_geti` | after first insertion pass, odd key lookup returns value other than `-2` | assertion abort | [x] |
| 31 | `hm_geti` | after first insertion pass, even key lookup returns value other than `i*5` | assertion abort | [x] |
| 32 | `hm_geti` | thread-safe lookup after first pass returns wrong default for odd key | assertion abort | [x] |
| 33 | `hm_geti` | thread-safe lookup after first pass returns value other than `i*5` for even key | assertion abort | [x] |
| 34 | `hm_geti` | after update pass, odd key lookup returns value other than `-2` | assertion abort | [x] |
| 35 | `hm_geti` | after update pass, even key lookup returns value other than `i*3` | assertion abort | [x] |
| 36 | `hm_geti` | after deleting keys `2 mod 4`, non-multiple-of-four lookup returns value other than `-2` | assertion abort | [x] |
| 37 | `hm_geti` | after partial deletion, multiple-of-four lookup returns value other than `i*3` | assertion abort | [x] |
| 38 | `hm_geti` | after deleting every key, any lookup returns value other than `-2` | assertion abort | [x] |
| 39 | all pointer-taking APIs | malformed non-null pointer, undersized element, invalid key pointer, or inconsistent map metadata | no C validation; behavior is undefined and may fault rather than returning an error code | [x] |
| 40 | length-taking APIs | zero lengths and lengths one past natural boundaries (`7/8`, capacity, block size) | accepted; branch-specific result must match C exactly | [x] |
