# Error surface

The C API has no error enum and no `RETURN_ERROR` macro. Its rejection surface
consists of `-1`/null sentinels and assertions. Rows below come from every
`return -1`, public null/absent-key branch, and every `STBDS_ASSERT` in
`src/lib.c`. Assertions in private helpers are invariant checks rather than
documented caller errors, but are included so they are not hidden.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `stbds_hmget_key_ts` | `a == NULL` (empty map lookup) | allocate the zero default element, return a non-null hash pointer, and store `-1` in `*temp` | [x] |
| 2 | `stbds_hmget_key_ts` | map exists but its hash table pointer is null | return the same pointer and store `-1` in `*temp` | [x] |
| 3 | `stbds_hmget_key_ts` / `stbds_hmget_key` | key is absent and the first probe scan reaches an empty slot | return the same map and report index `-1` (`*temp` or header temp) | [x] |
| 4 | `stbds_hmget_key_ts` / `stbds_hmget_key` | key is absent and the wrapped probe scan reaches an empty slot | return the same map and report index `-1` (`*temp` or header temp) | [x] |
| 5 | `stbds_hmdel_key` | `a == NULL` | return null | [x] |
| 6 | `stbds_hmdel_key` | map exists but its hash table pointer is null | return the same map and set header temp to `0` | [x] |
| 7 | `stbds_hmdel_key` | requested key is absent | return the same map unchanged and set header temp to `0` | [x] |
| 8 | `stbds_make_hash_index` (private, reached by map APIs) | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort; public constructors use powers of two starting at 8, so malformed internal state is required | [x] |
| 9 | `stbds_hmput_key` | after growth, `new_length > capacity` | `assert` abort; allocator failure/corrupt array metadata is required | [x] |
| 10 | `stbds_hmdel_key` | located slot is outside `table->slot_count` | `assert` abort; corrupt hash metadata is required | [x] |
| 11 | `stbds_hmdel_key` | decrementing used count would violate the internal nonnegative-count invariant | `assert` abort; corrupt hash metadata is required | [x] |
| 12 | `stbds_hmdel_key` | moved final element cannot be found in the hash index | `assert` abort; corrupt hash metadata is required | [x] |
| 13 | `stbds_hmdel_key` | moved element's index is not the old final index | `assert` abort; corrupt hash metadata is required | [x] |
| 14 | `stbds_stralloc` | selected arena block still has `len > remaining` | `assert` abort; corrupt arena metadata/allocation failure is required | [x] |
| 15 | `intput` | after inserts, lookup of key `9` differs from `num` | `assert` abort | [x] |
| 16 | `intput` | after inserts, lookup of key `11` differs from `3` | `assert` abort | [x] |
| 17 | `intput` | after inserts, lookup of key `num` differs from `7` (externally reachable for `num == 9` or `num == 11`) | `assert` abort | [x] |

Phase C status: all externally constructible rows must be checked by
`tests/differential.rs`; private invariant rows are checked by operations that
exercise and preserve those exact invariants without corrupting allocator-owned
memory.
