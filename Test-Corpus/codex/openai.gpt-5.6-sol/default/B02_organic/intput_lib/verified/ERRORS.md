# Error Surface

This table is mechanically derived from the explicit sentinel returns and
`STBDS_ASSERT` sites in `src/lib.c`. The library has no error enum or
`RETURN_ERROR` macro. Rows marked "internal invariant" have no direct public
argument that makes a well-formed call violate the condition; they are still
listed because the C source asserts them.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `stbds_hmdel_key` | `a == NULL` | returns `NULL` | [x] |
| 2 | `stbds_hmdel_key` | array exists but `header->hash_table == NULL` | returns the original pointer and stores `temp = 0` | [x] |
| 3 | `stbds_hmdel_key` | requested key is absent (`stbds_hm_find_slot < 0`) | returns the original pointer and stores `temp = 0` | [x] |
| 4 | `stbds_hmget_key_ts` | `a == NULL` (empty-map lookup) | creates the zero default element, stores `temp = -1`, and returns the hash-array pointer | [x] |
| 5 | `stbds_hmget_key_ts` | array exists but has no hash table | stores `temp = -1` and returns the original pointer | [x] |
| 6 | `stbds_hmget_key_ts` | requested key is absent | stores `temp = -1` and returns the original pointer | [x] |
| 7 | `stbds_hmfree_func` | `a == NULL` | returns immediately (no-op) | [x] |
| 8 | `stbds_make_hash_index` (via map creation/growth) | internal invariant `used_count_threshold + tombstone_count_threshold >= slot_count` | assertion failure (`SIGABRT`) | [x] |
| 9 | `stbds_hmput_key` | internal invariant after growth `(size_t)i + 1 > arrcap(a)` | assertion failure (`SIGABRT`) | [x] |
| 10 | `stbds_hmdel_key` | internal invariant `slot >= table->slot_count` | assertion failure (`SIGABRT`) | [x] |
| 11 | `stbds_hmdel_key` | internal invariant `table->used_count < 0` (the field is unsigned, so this condition is not externally constructible) | assertion failure (`SIGABRT`) | [x] |
| 12 | `stbds_hmdel_key` | moved final element cannot be found (`slot < 0`) | assertion failure (`SIGABRT`) | [x] |
| 13 | `stbds_hmdel_key` | moved element's bucket index differs from `final_index` | assertion failure (`SIGABRT`) | [x] |
| 14 | `stbds_stralloc` | internal invariant after block allocation `len > a->remaining` | assertion failure (`SIGABRT`) | [x] |
| 15 | `intput` | postcondition `hmget(intmap, 9) != num` | assertion failure (`SIGABRT`); unreachable for an uncorrupted map | [x] |
| 16 | `intput` | postcondition `hmget(intmap, 11) != 3` | assertion failure (`SIGABRT`); unreachable for an uncorrupted map | [x] |
| 17 | `intput` | postcondition `hmget(intmap, num) != 7`; externally triggered by `num == 9` or `num == 11` because a later insertion updates the same key | assertion failure (`SIGABRT`) | [x] |

Generic FFI boundaries are tested in addition to these source-derived rows:
null pointers, zero lengths, oversized lengths that can be exercised without
unbounded allocation, and out-of-range integer mode values.
