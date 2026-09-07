# Error surface

The C API has no error enum and no `RETURN_ERROR`/`return -1` public return
contract. Rejection is represented by null/no-op returns, `temp == -1`, or an
assertion abort. Rows below come from every null/missing-key branch and every
`STBDS_ASSERT` in `src/lib.c`. Generic malformed-pointer boundaries are listed
last even where C's result is process termination rather than a sentinel.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `stbds_hmfree_func` | `a == NULL` | return immediately; no-op | [x] |
| 2 | `stbds_hmget_key_ts` | `a == NULL` (empty map lookup) | allocate default element and set `*temp = -1` | [x] |
| 3 | `stbds_hmget_key_ts` | non-null map whose header has `hash_table == NULL` | return same map and set `*temp = -1` | [x] |
| 4 | `stbds_hmget_key_ts` | hash table exists but key is absent; empty slot encountered in the first probe-range scan | return same map and set `*temp = -1` | [x] |
| 5 | `stbds_hmget_key_ts` | hash table exists but key is absent; empty slot encountered in the wrapped probe-range scan | return same map and set `*temp = -1` | [x] |
| 6 | `stbds_hmget_key` | key is absent (the `hmget_key_ts` rejection path) | return map and store header `temp = -1` | [x] |
| 7 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 8 | `stbds_hmdel_key` | non-null map whose header has `hash_table == NULL` | return same map; header `temp = 0` | [x] |
| 9 | `stbds_hmdel_key` | hash table exists but key is absent | return same map; header `temp = 0` | [x] |
| 10 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort | [x] |
| 11 | `stbds_hmput_key` | post-growth invariant `(size_t)i + 1 > arrcap(a)` | `assert` abort | [x] |
| 12 | `stbds_hmdel_key` | located slot is outside `table->slot_count` | `assert` abort | [x] |
| 13 | `stbds_hmdel_key` | source assertion checks `table->used_count >= 0` after decrement | assertion always passes because the field is unsigned, including after wrap | [x] |
| 14 | `stbds_hmdel_key` | moved final entry cannot be found after compaction (`slot < 0`) | `assert` abort | [x] |
| 15 | `stbds_hmdel_key` | moved entry's bucket index is not `final_index` | `assert` abort | [x] |
| 16 | `stbds_stralloc` | normal-block path reaches `len > a->remaining` after allocation | `assert` abort | [x] |
| 17 | `str_dups` | duplicated key's first byte is not `'a'` | `assert` abort | [x] |
| 18 | `str_dups` | strdup mode preserves the source key pointer instead of duplicating it | `assert` abort | [x] |
| 19 | `str_dups` | inserted value differs from `num` | `assert` abort | [x] |
| 20 | `stbds_hash_string` | `str == NULL` | invalid dereference; process fault | [x] |
| 21 | `stbds_hash_bytes` | `p == NULL && len > 0` | invalid dereference; process fault | [x] |
| 22 | `stbds_arrfreef` | `a == NULL` | header-before-pointer invalid free; process fault/abort | [x] |
| 23 | map key APIs | `key == NULL` with nonzero key size in binary mode, or any string mode | invalid dereference in hash/compare; process fault | [x] |
| 24 | `stbds_hmget_key_ts` | `temp == NULL` on a path that writes it | invalid dereference; process fault | [x] |
| 25 | `stbds_stralloc` / `stbds_strreset` | `arena == NULL` | invalid dereference; process fault | [x] |
| 26 | `stbds_stralloc` | `str == NULL` | invalid dereference in `strlen`; process fault | [x] |
| 27 | allocation APIs | size arithmetic overflows or allocator returns `NULL` | C performs unchecked pointer arithmetic/dereference; process fault or allocator abort | [x] |
| 28 | mode-taking map APIs | enum-like `mode` is outside documented `0` (binary) / `1` (string) range | C accepts any `int`: values `< 1` take binary branches and values `>= 1` take string branches; deletion frees strdup keys only when `mode == 1` | [x] |
