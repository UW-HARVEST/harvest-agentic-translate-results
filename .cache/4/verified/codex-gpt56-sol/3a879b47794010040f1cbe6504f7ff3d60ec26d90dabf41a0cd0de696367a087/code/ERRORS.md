# Error Surface

This table is derived from every `return -1`/null rejection branch and every
`STBDS_ASSERT` in `c_src/src/lib.c`. The C API does not define an error enum.
Allocation failures and invalid non-null pointers are unchecked and therefore
have undefined behavior rather than a C rejection result.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|----------------------------------------------|-------------------|---------|
| 1 | `stbds_hm_find_slot` via get/delete | first probe segment reaches `STBDS_HASH_EMPTY` before a matching key | internal `-1`; caller reports missing key | [x] |
| 2 | `stbds_hm_find_slot` via get/delete | wrapped probe segment reaches `STBDS_HASH_EMPTY` before a matching key | internal `-1`; caller reports missing key | [x] |
| 3 | `stbds_hmget_key_ts` | `a == NULL` | allocate default entry, set `*temp = -1`, return non-null map pointer | [x] |
| 4 | `stbds_hmget_key_ts` | map exists but `header->hash_table == NULL` | set `*temp = -1`, return the same pointer | [x] |
| 5 | `stbds_hmget_key_ts` | hash table exists but key is absent | set `*temp = -1`, return the same pointer | [x] |
| 6 | `stbds_hmget_key` | key is absent | return map and store `header->temp = -1` | [x] |
| 7 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 8 | `stbds_hmdel_key` | map exists but `header->hash_table == NULL` | return same map and store `header->temp = 0` | [x] |
| 9 | `stbds_hmdel_key` | hash table exists but key is absent | return same map and store `header->temp = 0` | [x] |
| 10 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | assertion failure (`SIGABRT`); invariant-only static helper check | [x] |
| 11 | `stbds_hmput_key` | post-growth `i + 1 > arrcap(a)` | assertion failure (`SIGABRT`); allocation/postcondition invariant | [x] |
| 12 | `stbds_hmdel_key` | found slot is not less than `table->slot_count` | assertion failure (`SIGABRT`); lookup invariant | [x] |
| 13 | `stbds_hmdel_key` | `table->used_count == 0` before decrement | wraps to `SIZE_MAX`; `used_count >= 0` is always true because the field is unsigned, so no assertion failure | [x] |
| 14 | `stbds_hmdel_key` | moved final element cannot be found after compaction (`slot < 0`), including non-last deletion with out-of-range `mode = 2` | assertion failure (`SIGABRT`) | [x] |
| 15 | `stbds_hmdel_key` | moved element's bucket index is not `final_index` | assertion failure (`SIGABRT`); table-corruption invariant | [x] |
| 16 | `stbds_stralloc` | after block allocation, `len > a->remaining` | assertion failure (`SIGABRT`); allocation/postcondition invariant | [x] |
| 17 | `str_put` | inserted key's first byte is not `'a'` | assertion failure (`SIGABRT`); composed-operation invariant | [x] |
| 18 | `str_put` | inserted key pointer differs from source pointer | assertion failure (`SIGABRT`); composed-operation invariant | [x] |
| 19 | `str_put` | inserted value differs from `num` | assertion failure (`SIGABRT`); composed-operation invariant | [x] |

Generic FFI boundaries to exercise in addition to the source checks:

| # | function(s) | boundary | expected C behavior | covered |
|---|-------------|----------|---------------------|---------|
| G1 | `stbds_hash_bytes` | null pointer with zero length | valid hash result; pointer is not dereferenced | [x] |
| G2 | pointer-taking APIs | null pointer where the C source explicitly checks it | exact sentinel/no-op listed above | [x] |
| G3 | length-taking APIs | zero lengths and zero element sizes | exact return/pointer metadata produced by C | [x] |
| G4 | length-taking APIs | oversized `size_t` values | C unsigned wraparound behavior where no allocation/dereference is required | [x] |
| G5 | hash-map APIs | `mode = -1` and `mode = 2` (outside documented `0..=1`) | `< 1` follows binary branches; `>= 1` follows string branches | [x] |

Rows 11, 16, and 17-19 are postcondition assertions whose false conditions
cannot be supplied through the public ABI: no input is read between the
operation and assertion. Their coverage executes the assertions across the
randomized growth, arena, and `str_put` matrices. Rows 10, 12, 14, and 15 use
isolated child processes to construct corrupted internal state and compare the
resulting `SIGABRT`; row 13 explicitly verifies the unsigned wraparound.
