# Error Surface

This table is mechanically derived from every sentinel return, explicit null
branch, and `STBDS_ASSERT` in `c_src/src/lib.c`. The library defines no error
enum and returns no conventional error code. Assertions numbered 8-26 are
internal consistency guards: their failure conditions cannot be produced by a
well-formed public call without first corrupting library-owned state (and row
11 is tautologically false because `used_count` is unsigned).

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|----------------------------------------------|-------------------|---------|
| 1 | `stbds_hmfree_func` | `a == NULL` | Return immediately; no effect | [x] |
| 2 | `stbds_hmdel_key` | `a == NULL` | Return `NULL` | [x] |
| 3 | `stbds_hm_find_slot` via get/delete | Probe reaches an empty slot in the first bucket span before finding the key | Return index `-1` | [x] |
| 4 | `stbds_hm_find_slot` via get/delete | Probe wraps and reaches an empty slot in the second bucket span before finding the key | Return index `-1` | [x] |
| 5 | `stbds_hmget_key_ts` | Map exists but its hash table is `NULL` | Store `-1` in `*temp`; return map | [x] |
| 6 | `stbds_hmget_key_ts` | Hash table exists but key lookup returns a negative slot | Store `-1` in `*temp`; return map | [x] |
| 7 | `stbds_hmdel_key` | Map has no hash table, or requested key lookup returns a negative slot | Return map unchanged and leave header `temp == 0` | [x] |
| 8 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort | [x] |
| 9 | `stbds_hmput_key` | After attempted growth, `length + 1 > capacity` | `assert` abort | [x] |
| 10 | `stbds_hmdel_key` | Found slot is not below `table->slot_count` | `assert` abort | [x] |
| 11 | `stbds_hmdel_key` | Decremented unsigned `table->used_count < 0` | `assert` abort; condition is unreachable for `size_t` | [x] |
| 12 | `stbds_hmdel_key` | Moved final entry cannot be found after compaction (`slot < 0`) | `assert` abort | [x] |
| 13 | `stbds_hmdel_key` | Moved entry's bucket index does not equal the former final index | `assert` abort | [x] |
| 14 | `stbds_stralloc` | Standard-block path reaches copy with `len > arena->remaining` | `assert` abort | [x] |
| 15 | `hm_geti` | Initial lookup in a null map does not produce index `-1` | `assert` abort | [x] |
| 16 | `hm_geti` | Lookup after setting the default does not produce index `-1` | `assert` abort | [x] |
| 17 | `hm_geti` | Missing key after setting the default does not return `-2` | `assert` abort | [x] |
| 18 | `hm_geti` | First-pass odd-key lookup does not return `-2` | `assert` abort | [x] |
| 19 | `hm_geti` | First-pass even-key lookup does not return `key * 5` | `assert` abort | [x] |
| 20 | `hm_geti` | Thread-safe first-pass odd-key lookup does not return `-2` | `assert` abort | [x] |
| 21 | `hm_geti` | Thread-safe first-pass even-key lookup does not return `key * 5` | `assert` abort | [x] |
| 22 | `hm_geti` | Updated odd-key lookup does not return `-2` | `assert` abort | [x] |
| 23 | `hm_geti` | Updated even-key lookup does not return `key * 3` | `assert` abort | [x] |
| 24 | `hm_geti` | Post-selective-delete lookup where `(key & 3) != 0` does not return `-2` | `assert` abort | [x] |
| 25 | `hm_geti` | Post-selective-delete lookup where `(key & 3) == 0` does not return `key * 3` | `assert` abort | [x] |
| 26 | `hm_geti` | Lookup after deleting every requested key does not return `-2` | `assert` abort | [x] |

## Unchecked Undefined Inputs

The following are not C rejection branches and therefore are not rows in the
table: null pointers passed to functions that immediately dereference them,
nonzero lengths paired with invalid buffers, integer allocation-size overflow,
and allocation failure. The C behavior for those inputs is undefined rather
than an error result.
