# Error surface

The C library has no error enum and no `RETURN_ERROR` macro. It mostly uses
sentinels for absent data and `assert` for internal data-structure invariants.
Rows below are mechanically derived from every sentinel-return branch, null
rejection/no-op, explicit mode boundary, and assertion in `src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `stbds_hmfree_func` | `a == NULL` | return normally without doing anything | [x] |
| 2 | `stbds_hmget_key_ts` | `a == NULL` | allocate a one-element zero default record, set `*temp = -1`, return the public hash pointer | [x] |
| 3 | `stbds_hmget_key_ts` | backing array exists but `hash_table == NULL` | set `*temp = -1`, return `a` unchanged | [x] |
| 4 | `stbds_hmget_key_ts` | hash table exists but key is absent (`stbds_hm_find_slot < 0`) | set `*temp = -1`, return `a` unchanged | [x] |
| 5 | `stbds_hmget_key` | key is absent | return the same pointer and store `-1` in the backing header's `temp` field | [x] |
| 6 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 7 | `stbds_hmdel_key` | backing array exists but `hash_table == NULL` | set header `temp = 0`, return `a` unchanged | [x] |
| 8 | `stbds_hmdel_key` | hash table exists but key is absent | set header `temp = 0`, return `a` unchanged | [x] |
| 9 | hash-map mode boundary | `mode < STBDS_HM_STRING` (including negative/out-of-range enum values) | treat key as binary bytes | [x] |
| 10 | hash-map mode boundary | `mode >= STBDS_HM_STRING` (including out-of-range enum values) | treat key as a NUL-terminated string | [x] |
| 11 | `stbds_shmode_func` / insertion switch | mode is outside `STBDS_SH_NONE..STBDS_SH_ARENA` after conversion to `unsigned char` | use the insertion switch's `default` branch and copy `keysize` bytes | [x] |
| 12 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | assertion failure/abort (internal invariant; not reachable for the library's generated power-of-two slot counts) | [x] |
| 13 | `stbds_hmput_key` | growth returns capacity smaller than `i + 1` | assertion failure/abort (allocation/internal invariant) | [x] |
| 14 | `stbds_hmdel_key` | located slot is outside `table->slot_count` | assertion failure/abort (corrupt internal table) | [x] |
| 15 | `stbds_hmdel_key` | decrement would make `used_count` invalid | assertion failure/abort (the C field is unsigned, so the literal check is tautological after normal operations) | [x] |
| 16 | `stbds_hmdel_key` | moving the final entry cannot find that entry's hash slot | assertion failure/abort (corrupt internal table) | [x] |
| 17 | `stbds_hmdel_key` | moved entry's hash slot does not point at `final_index` | assertion failure/abort (corrupt internal table) | [x] |
| 18 | `stbds_stralloc` | post-allocation `len > a->remaining` | assertion failure/abort (allocation/internal invariant) | [x] |

Public calls requiring non-null storage (`stbds_arrfreef`, nonzero-length
`stbds_hash_bytes`, `stbds_hash_string`, `stbds_stralloc`, and
`stbds_strreset`) perform no C validation. Passing null there is undefined
behavior rather than a defined rejection result, so no portable error sentinel
exists to compare.

Rows 12-18 are internal invariants rather than constructible public-input
states. The randomized operations exercise each invariant site without either
implementation rejecting valid state; forcing those conditions would require
allocator failure or deliberate corruption.
