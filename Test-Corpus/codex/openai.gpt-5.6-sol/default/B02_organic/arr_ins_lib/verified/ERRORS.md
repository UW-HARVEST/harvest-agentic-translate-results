# Error surface

The C API has no error enum and no `RETURN_ERROR`/`return NULL` validation
framework. It assumes valid allocation sizes, element layouts, and pointers.
The rows below enumerate every distinct sentinel/rejection branch and every
active `STBDS_ASSERT` in `src/lib.c`. Internal assertions are invariants rather
than supported invalid-input contracts; their reachable public operation is
listed so valid randomized tests exercise the invariant.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `stbds_hmget_key_ts` | `a == NULL` (empty map lookup) | Allocate the one-element default entry, set `*temp = -1`, return hash-view pointer | [x] |
| 2 | `stbds_hmget_key_ts` | map storage exists but `header->hash_table == NULL` | Set `*temp = -1`, return the input pointer unchanged | [x] |
| 3 | `stbds_hmget_key_ts` / `stbds_hmget_key` | key is absent and probing reaches an empty hash slot | Set temp/header temp to `-1`, return map unchanged | [x] |
| 4 | `stbds_hmdel_key` | `a == NULL` | Return `NULL` | [x] |
| 5 | `stbds_hmdel_key` | map storage exists but `header->hash_table == NULL` | Set header temp to `0`, return map unchanged | [x] |
| 6 | `stbds_hmdel_key` | key is absent (`stbds_hm_find_slot < 0`) | Set header temp to `0`, return map unchanged | [x] |
| 7 | `stbds_make_hash_index` assertion | generated thresholds violate `used_count_threshold + tombstone_count_threshold < slot_count` | C assertion failure; all public-generated slot counts (8 and powers of two) must satisfy it | [x] |
| 8 | `stbds_hmput_key` assertion | post-growth `i + 1 > arrcap(a)` | C assertion failure; successful insertion must maintain `i + 1 <= capacity` | [x] |
| 9 | `stbds_hmdel_key` assertion | located `slot >= table->slot_count` | C assertion failure; every successful deletion must locate an in-range slot | [x] |
| 10 | `stbds_hmdel_key` assertion | decrement would make `used_count < 0` | C assertion failure (the C field is unsigned, so valid operations must never underflow it) | [x] |
| 11 | `stbds_hmdel_key` assertion | moved final element cannot be found (`slot < 0`) | C assertion failure; deleting a non-final entry must find the moved key | [x] |
| 12 | `stbds_hmdel_key` assertion | moved key's hash slot does not contain `final_index` | C assertion failure; moved-entry index must be repaired from `final_index` to `old_index` | [x] |
| 13 | `stbds_stralloc` assertion | after block selection/allocation, `len > arena->remaining` on the normal (non-oversized) path | C assertion failure; valid arena growth must leave enough remaining bytes | [x] |
| 14 | `arr_ins` assertion | inserted array element at index `i` differs from `num` | C assertion failure; insertion must preserve the inserted value | [x] |
| 15 | `arr_ins` assertion | for `i < 4`, shifted element at index 4 differs from `4` | C assertion failure; insertion shift must preserve the previous fourth element | [x] |
| 16 | mode-taking map APIs | mode is one below the valid binary mode (`-1`) | C treats it as binary (`mode < 1`), not as a rejected enum | [x] |
| 17 | mode-taking map APIs / `stbds_shmode_func` | mode is one above the declared string-arena mode (`4`) | C accepts the integer; `mode >= 1` selects string hashing while stored mode `4` takes the switch default copy branch | [x] |
| 18 | `stbds_hash_bytes` | `p == NULL` and `len == 0` | Return the zero-length hash without dereferencing `p` | [x] |
| 19 | `stbds_hmfree_func` | `a == NULL` | Return without action | [x] |
| 20 | `stbds_arrgrowf` | `a == NULL`, `addlen == 0`, `min_cap == 0` | Return `NULL` unchanged | [x] |

Unsupported pointer/length pairs such as `p == NULL && len > 0`, null strings,
null output pointers, fabricated internal headers, and arithmetic-overflowing
allocation sizes have undefined behavior in C. They are recorded here as
outside the callable contract rather than assigned invented error results.

