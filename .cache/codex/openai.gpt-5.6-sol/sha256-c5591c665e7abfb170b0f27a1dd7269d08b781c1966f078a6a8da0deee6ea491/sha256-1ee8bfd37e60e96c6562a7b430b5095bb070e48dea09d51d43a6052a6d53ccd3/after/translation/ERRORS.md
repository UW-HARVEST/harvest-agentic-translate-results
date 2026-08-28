# Error surface

The C API has no error enum and no conventional error-code-returning public
function. Its explicit rejection surface consists of null/sentinel branches,
miss sentinels, and internal assertions. "Process fault" below is the C
behavior for invalid pointers; these are not converted into a Rust error.

| # | function | trigger (the exact invalid input/condition) | expected C result | Verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `stbds_arrgrowf` | requested allocation size cannot be represented/allocated (for example `elemsize=SIZE_MAX`, `min_cap=SIZE_MAX`) | unchecked `realloc` result is used; process faults | [x] |
| 2 | `stbds_arrfreef` | `a == NULL` | subtracts the header size and passes the invalid address to `free`; process faults/aborts | [x] |
| 3 | `stbds_hash_string` | `str == NULL` | dereferences `str`; process faults | [x] |
| 4 | `stbds_hash_bytes` | `p == NULL && len == 0` | accepted; returns the empty-input hash | [x] |
| 5 | `stbds_hash_bytes` | `p == NULL && len > 0` | dereferences `p`; process faults | [x] |
| 6 | `stbds_hmfree_func` | `a == NULL` | explicit no-op return | [x] |
| 7 | `stbds_hmget_key_ts` | `a == NULL` | creates a default entry, writes `STBDS_INDEX_EMPTY` (`-1`) to `*temp`, returns map pointer | [x] |
| 8 | `stbds_hmget_key_ts` | `temp == NULL` | writes through `temp`; process faults | [x] |
| 9 | `stbds_hmget_key_ts` / `stbds_hmget_key` | map has no table or key is absent | reports `STBDS_INDEX_EMPTY` (`-1`) in `temp`/header temp | [x] |
| 10 | `stbds_hmdel_key` | `a == NULL` | explicit `NULL` return | [x] |
| 11 | `stbds_hmdel_key` | map exists but `hash_table == NULL` | returns original map; header temp is `0` | [x] |
| 12 | `stbds_hmdel_key` | hash table exists but key is absent (`stbds_hm_find_slot < 0`) | returns original map; header temp is `0` | [x] |
| 13 | `stbds_stralloc` | `a == NULL` | dereferences arena; process faults | [x] |
| 14 | `stbds_stralloc` | `str == NULL` | `strlen(NULL)` faults | [x] |
| 15 | `stbds_strreset` | `a == NULL` | dereferences arena; process faults | [x] |
| 16 | `stbds_make_hash_index` (internal) | `used_count_threshold + tombstone_count_threshold >= slot_count` | `STBDS_ASSERT` aborts; impossible for API-created power-of-two tables | [x] |
| 17 | `stbds_hmput_key` | after growth, `i + 1 > array capacity` | `STBDS_ASSERT` aborts; only reachable after allocator/state corruption | [x] |
| 18 | `stbds_hmdel_key` | found slot is outside `table->slot_count` | `STBDS_ASSERT` aborts; only reachable after table corruption | [x] |
| 19 | `stbds_hmdel_key` | `table->used_count < 0` after decrement | assertion is tautologically true because `used_count` is `size_t`; no rejecting input exists | [x] |
| 20 | `stbds_hmdel_key` | moved final element cannot be found (`slot < 0`) | `STBDS_ASSERT` aborts; only reachable after table corruption | [x] |
| 21 | `stbds_hmdel_key` | moved element's bucket index is not `final_index` | `STBDS_ASSERT` aborts; only reachable after table corruption | [x] |
| 22 | `stbds_stralloc` | post-allocation `len > a->remaining` | `STBDS_ASSERT` aborts; only reachable after allocator/state corruption | [x] |
| 23 | `str_put` | inserted key's first byte is not `'a'` | `STBDS_ASSERT` aborts; internal postcondition | [x] |
| 24 | `str_put` | stored default-mode key pointer differs from input pointer | `STBDS_ASSERT` aborts; internal postcondition | [x] |
| 25 | `str_put` | stored value differs from `num` | `STBDS_ASSERT` aborts; internal postcondition | [x] |

No public C function checks `elemsize`, `keysize`, `keyoffset`, string
termination, or allocation failure. Inputs violating those requirements have
undefined memory behavior rather than a C error result.
