# Error surface

The C API has no error enum and no `RETURN_ERROR` macro. Its recoverable
rejections use null/unchanged pointers or index `-1`; invariant violations use
`assert` and abort. Rows 7-13 are internal invariants reachable only after
allocator failure or caller memory corruption, not ordinary well-formed FFI
inputs. Rows 14-25 are the assertions in the exported `hm_geti` self-test.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|---|
| 1 | `stbds_hmget_key_ts` | `a == NULL` (empty map lookup) | allocate the default element, set `*temp = -1`, return map pointer | [x] |
| 2 | `stbds_hmget_key_ts` | map exists but `hash_table == NULL` | set `*temp = -1`, return the unchanged map pointer | [x] |
| 3 | `stbds_hm_find_slot` via get/delete | suffix scan reaches `STBDS_HASH_EMPTY` before finding key | return slot `-1`; public get reports index `-1`, delete reports not deleted | [x] |
| 4 | `stbds_hm_find_slot` via get/delete | wrapped prefix scan reaches `STBDS_HASH_EMPTY` before finding key | return slot `-1`; public get reports index `-1`, delete reports not deleted | [x] |
| 5 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 6 | `stbds_hmdel_key` | map exists but has no hash table, or key is absent | return unchanged pointer and leave header `temp = 0` | [x] |
| 7 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | assertion failure (`SIGABRT`) | [x] |
| 8 | `stbds_hmput_key` | post-growth `i + 1 > capacity` | assertion failure (`SIGABRT`) | [x] |
| 9 | `stbds_hmdel_key` | located `slot >= table->slot_count` | assertion failure (`SIGABRT`) | [x] |
| 10 | `stbds_hmdel_key` | decrement would make `used_count < 0` | assertion failure (`SIGABRT`) | [x] |
| 11 | `stbds_hmdel_key` | moved final element cannot be found (`slot < 0`) | assertion failure (`SIGABRT`) | [x] |
| 12 | `stbds_hmdel_key` | moved element's bucket index is not `final_index` | assertion failure (`SIGABRT`) | [x] |
| 13 | `stbds_stralloc` | normal-block path ends with `len > remaining` | assertion failure (`SIGABRT`) | [x] |
| 14 | `hm_geti` | initial empty lookup does not return `-1` | assertion failure (`SIGABRT`) | [x] |
| 15 | `hm_geti` | lookup after setting default does not return `-1` | assertion failure (`SIGABRT`) | [x] |
| 16 | `hm_geti` | missing key does not return default value `-2` | assertion failure (`SIGABRT`) | [x] |
| 17 | `hm_geti` | odd key after first insertion pass does not return `-2` | assertion failure (`SIGABRT`) | [x] |
| 18 | `hm_geti` | even key after first insertion pass does not return `i * 5` | assertion failure (`SIGABRT`) | [x] |
| 19 | `hm_geti` | thread-safe odd-key lookup does not return `-2` | assertion failure (`SIGABRT`) | [x] |
| 20 | `hm_geti` | thread-safe even-key lookup does not return `i * 5` | assertion failure (`SIGABRT`) | [x] |
| 21 | `hm_geti` | odd key after update pass does not return `-2` | assertion failure (`SIGABRT`) | [x] |
| 22 | `hm_geti` | even key after update pass does not return `i * 3` | assertion failure (`SIGABRT`) | [x] |
| 23 | `hm_geti` | key not divisible by four after selective deletes does not return `-2` | assertion failure (`SIGABRT`) | [x] |
| 24 | `hm_geti` | key divisible by four after selective deletes does not return `i * 3` | assertion failure (`SIGABRT`) | [x] |
| 25 | `hm_geti` | any key remains after full delete pass | assertion failure (`SIGABRT`) | [x] |

Generic FFI boundaries that are undefined in C (for example a null string,
null output pointer, `stbds_arrfreef(NULL)`, or a positive length with a null
byte pointer) are not rejection paths: C dereferences invalid memory. They are
covered only where C defines a result (rows 1, 2, 5, and 6).

