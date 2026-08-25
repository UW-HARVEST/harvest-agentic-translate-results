# Error Surface

The C API has no error enum or `RETURN_ERROR` macro. Its observable rejection
surface consists of `-1` lookup sentinels, null/no-op returns, and internal
`assert` aborts. Rows 10-16 are internal representation invariants rather than
conditions reachable through a valid public call sequence.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `stbds_hm_find_slot` via `stbds_hmget_key_ts` | Probe from the initial position reaches an empty slot before finding the key | `temp = -1`; array pointer returned unchanged | [x] |
| 2 | `stbds_hm_find_slot` via `stbds_hmget_key_ts` | Wrapped portion of the initial bucket reaches an empty slot before finding the key | `temp = -1`; array pointer returned unchanged | [x] |
| 3 | `stbds_hmget_key_ts` | `a == NULL` | Allocate the zero default entry, set `temp = -1`, and return the hash-view pointer | [x] |
| 4 | `stbds_hmget_key_ts` | Array exists but its hash table is null | Set `temp = -1`; return the array pointer unchanged | [x] |
| 5 | `stbds_hmdel_key` | `a == NULL` | Return `NULL` | [x] |
| 6 | `stbds_hmdel_key` | Array exists but its hash table is null | Set header `temp = 0`; return the array unchanged | [x] |
| 7 | `stbds_hmdel_key` | Key is absent from a populated table | Set header `temp = 0`; return the array unchanged | [x] |
| 8 | `stbds_hmfree_func` | `a == NULL` | Return without action | [x] |
| 9 | `stbds_hash_bytes` | `p == NULL && len == 0` | Return the empty-input hash without dereferencing `p` | [x] |
| 10 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort | [x] |
| 11 | `stbds_hmput_key` | After growth, `i + 1 > array capacity` | `assert` abort; impossible after a successful `stbds_arrgrowf` by its capacity postcondition | [x] |
| 12 | `stbds_hmdel_key` | Located `slot >= table->slot_count` | `assert` abort | [x] |
| 13 | `stbds_hmdel_key` | Decremented `used_count < 0` | Never rejects: `used_count` is unsigned, so the assertion is true for every representation | [x] |
| 14 | `stbds_hmdel_key` | Moved final element cannot be found in the hash index (`slot < 0`) | `assert` abort | [x] |
| 15 | `stbds_hmdel_key` | Moved final element's bucket index is not `final_index` | `assert` abort | [x] |
| 16 | `stbds_stralloc` | Normal-block path finishes with `len > a->remaining` | `assert` abort; impossible because an undersized block takes the dedicated-block early return | [x] |
