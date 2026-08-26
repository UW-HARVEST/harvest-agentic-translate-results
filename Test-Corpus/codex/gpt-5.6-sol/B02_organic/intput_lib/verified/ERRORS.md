# Error Surface

The C implementation has no error enum or `RETURN_ERROR` macro. Its explicit
rejections are `-1` lookup sentinels, null/no-table delete sentinels, and
assertions. Rows 10-16 are internal consistency assertions: public callers
cannot directly supply the private state named in their triggers, so the tests
exercise the operations that reach and preserve those invariants.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `stbds_hmget_key_ts` | `a == NULL` (no map exists) | allocate the default entry and write `temp = -1` | [x] |
| 2 | `stbds_hmget_key_ts` | map exists but `header.hash_table == NULL` | return the input map and write `temp = -1` | [x] |
| 3 | `stbds_hmget_key_ts` | key search reaches an empty slot in the first bucket segment | return the input map and write `temp = -1` | [x] |
| 4 | `stbds_hmget_key_ts` | key search wraps and reaches an empty slot in the second bucket segment | return the input map and write `temp = -1` | [x] |
| 5 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 6 | `stbds_hmdel_key` | map exists but `header.hash_table == NULL` | return the input map with `header.temp = 0` | [x] |
| 7 | `stbds_hmdel_key` | requested key is absent (`stbds_hm_find_slot < 0`) | return the input map with `header.temp = 0` | [x] |
| 8 | `stbds_hmfree_func` | `a == NULL` | return normally without action | [x] |
| 9 | `stbds_make_hash_index` via map creation/growth | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort; generated slot counts must keep this false | [x] |
| 10 | `stbds_hmput_key` | post-growth `(size_t)i + 1 > arrcap(a)` | `assert` abort; successful growth must keep this false | [x] |
| 11 | `stbds_hmdel_key` | found `slot >= table->slot_count` | `assert` abort; lookup must produce an in-range slot | [x] |
| 12 | `stbds_hmdel_key` | post-decrement `table->used_count < 0` | assertion is vacuously true because `used_count` is `size_t` | [x] |
| 13 | `stbds_hmdel_key` | moved final element cannot be found (`slot < 0`) | `assert` abort | [x] |
| 14 | `stbds_hmdel_key` | moved final element's bucket index is not `final_index` | `assert` abort | [x] |
| 15 | `stbds_stralloc` | normal-block path leaves `len > a->remaining` | `assert` abort | [x] |
| 16 | `intput` | lookup of key `9` differs from `num` | `assert` abort; unreachable with uncorrupted map operations | [x] |
| 17 | `intput` | lookup of key `11` differs from `3` | `assert` abort; unreachable with uncorrupted map operations | [x] |
| 18 | `intput` | lookup of key `num` differs from `7`; concretely `num == 9 || num == 11` | `assert` abort | [x] |

Generic FFI boundary cases are tested in addition to the mechanically found
rows: null data with zero length, null data with nonzero length in subprocesses,
zero element/key sizes where C defines a result, oversized lengths in
subprocesses, and mode values below and above the declared constants.
