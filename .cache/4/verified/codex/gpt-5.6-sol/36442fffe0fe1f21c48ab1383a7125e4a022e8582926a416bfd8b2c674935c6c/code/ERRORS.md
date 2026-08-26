# Error Surface

The C API has no error enum and performs no allocation-failure handling. Its
observable rejection surface consists of `-1`/null/no-op sentinels plus
assertion failures for corrupted internal state. Rows 1-9 are reachable through
ordinary calls. Rows 10-18 are internal-invariant assertions; rows 16-18 are
validated by `sh_puts`, while the others require state corruption to trigger.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `stbds_hmfree_func` | `a == NULL` | return immediately; no-op | [x] |
| 2 | `stbds_hmget_key_ts` | `a == NULL` | allocate the default entry, set `*temp = -1`, return map pointer | [x] |
| 3 | `stbds_hmget_key_ts` | map exists but `header->hash_table == NULL` | set `*temp = -1`, return the same map pointer | [x] |
| 4 | `stbds_hmget_key_ts` | key is absent and the first probe scan encounters `STBDS_HASH_EMPTY` | set `*temp = -1`, return the same map pointer | [x] |
| 5 | `stbds_hmget_key_ts` | key is absent and the wrapped probe scan encounters `STBDS_HASH_EMPTY` | set `*temp = -1`, return the same map pointer | [x] |
| 6 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 7 | `stbds_hmdel_key` | map exists but `header->hash_table == NULL` | return the same pointer with `header->temp = 0` | [x] |
| 8 | `stbds_hmdel_key` | key is absent and `stbds_hm_find_slot` returns `-1` | return the same pointer with `header->temp = 0` | [x] |
| 9 | `stbds_hash_bytes` | `p == NULL && len == 0` | return the hash of an empty byte sequence without dereferencing `p` | [x] |
| 10 | `stbds_make_hash_index` (via `stbds_hmput_key`) | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` fails; process aborts | [x] |
| 11 | `stbds_hmput_key` | after growth, `(size_t)i + 1 > arrcap(a)` | `assert` fails; process aborts | [x] |
| 12 | `stbds_hmdel_key` | located `slot >= table->slot_count` | `assert` fails; process aborts | [x] |
| 13 | `stbds_hmdel_key` | moving the final entry causes `stbds_hm_find_slot` to return `< 0` | `assert` fails; process aborts | [x] |
| 14 | `stbds_hmdel_key` | moved entry's bucket index is not `final_index` | `assert` fails; process aborts | [x] |
| 15 | `stbds_stralloc` | after selecting/allocating a normal block, `len > arena->remaining` | `assert` fails; process aborts | [x] |
| 16 | `sh_puts` | arena-backed insertion does not preserve the first key byte as `'a'` | `assert` fails; process aborts | [x] |
| 17 | `sh_puts` | arena-backed insertion leaves the stored key pointer equal to the string literal pointer | `assert` fails; process aborts | [x] |
| 18 | `sh_puts` | inserted map value differs from input `num` | `assert` fails; process aborts | [x] |

Raw null pointers paired with a nonzero size, null output pointers, corrupted
allocation pointers, arithmetic overflow sizes, and unterminated C strings are
not rejected by this C implementation. They invoke undefined behavior rather
than returning an API error and therefore have no stable C result to reproduce.
