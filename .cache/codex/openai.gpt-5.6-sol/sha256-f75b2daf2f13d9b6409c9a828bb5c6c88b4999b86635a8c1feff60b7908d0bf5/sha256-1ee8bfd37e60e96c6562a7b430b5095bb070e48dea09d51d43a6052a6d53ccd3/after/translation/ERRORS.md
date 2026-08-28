# Error Surface

Mechanically derived from `return -1`, null/sentinel return branches, and every
`STBDS_ASSERT` in `c_src/src/lib.c`. Assertions describe internal invariant
rejections; where valid public calls cannot construct a corrupt internal state,
the differential test reaches and checks the invariant on both implementations.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | assertion failure | [x] |
| 2 | `stbds_hm_find_slot` | matching hash/key is not found before an empty slot in the first bucket segment | returns `-1` | [x] |
| 3 | `stbds_hm_find_slot` | matching hash/key is not found before an empty slot in the wrapped bucket segment | returns `-1` | [x] |
| 4 | `stbds_hmget_key_ts` | `a == NULL` | returns a newly allocated default-only map and writes `-1` to `*temp` | [x] |
| 5 | `stbds_hmget_key_ts` | map exists but `hash_table == NULL` | returns the same map and writes `-1` to `*temp` | [x] |
| 6 | `stbds_hmget_key_ts` | hash table exists but key is absent (`slot < 0`) | returns the same map and writes `-1` to `*temp` | [x] |
| 7 | `stbds_hmput_key` | array growth still leaves `i + 1 > capacity` | assertion failure | [x] |
| 8 | `stbds_hmdel_key` | `a == NULL` | returns `NULL` | [x] |
| 9 | `stbds_hmdel_key` | map exists but `hash_table == NULL` | returns the same map with header `temp == 0` | [x] |
| 10 | `stbds_hmdel_key` | key is absent (`slot < 0`) | returns the same map with header `temp == 0` | [x] |
| 11 | `stbds_hmdel_key` | found slot is outside `table->slot_count` | assertion failure | [x] |
| 12 | `stbds_hmdel_key` | decrementing `used_count` makes it negative | assertion failure | [x] |
| 13 | `stbds_hmdel_key` | moved final element cannot be found (`slot < 0`) | assertion failure | [x] |
| 14 | `stbds_hmdel_key` | moved element's bucket index is not `final_index` | assertion failure | [x] |
| 15 | `stbds_stralloc` | standard-block path ends with `len > a->remaining` | assertion failure | [x] |
| 16 | `arr_ins` | inserted value at index `i` differs from `num` | assertion failure | [x] |
| 17 | `arr_ins` | for `i < 4`, element 4 differs from `4` after insertion | assertion failure | [x] |

## Generic FFI Boundaries

The C source does not reject invalid non-null buffer sizes, invalid element
sizes, null strings, null arenas, null output pointers, or arbitrary integer
modes. Such calls either have defined branch behavior recorded above, are
accepted as modes/configurations, or invoke C undefined behavior. Fatal
null-pointer cases are compared out of process so one library cannot terminate
the differential-test runner.
