# Error Surface

The C API does not define an error enum. Its recoverable rejection surface is
made of null/no-op returns and the `STBDS_INDEX_EMPTY` (`-1`) sentinel.
Assertions reject corrupted internal state by aborting.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `stbds_hmfree_func` | `a == NULL` | return without action | [x] |
| 2 | `stbds_hmget_key_ts` | `a == NULL` | allocate zero default entry, set `*temp = -1`, return map pointer | [x] |
| 3 | `stbds_hmget_key_ts` | map exists but `hash_table == NULL` | set `*temp = -1`, return same map pointer | [x] |
| 4 | `stbds_hmget_key_ts` | key is absent from a populated hash table | set `*temp = -1`, return same map pointer | [x] |
| 5 | `stbds_hmget_key` | key is absent (the `_ts` rejection paths) | return map and store `temp = -1` in its header | [x] |
| 6 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 7 | `stbds_hmdel_key` | map exists but `hash_table == NULL` | set header `temp = 0`, return same map | [x] |
| 8 | `stbds_hmdel_key` | key is absent from a populated hash table | set header `temp = 0`, return same map unchanged | [x] |
| 9 | `stbds_make_hash_index` (via put/mode) | threshold invariant `used_count_threshold + tombstone_count_threshold >= slot_count` | assertion failure/abort | [x] |
| 10 | `stbds_hmput_key` | post-growth invariant `i + 1 > arrcap(a)` | assertion failure/abort | [x] |
| 11 | `stbds_hmdel_key` | found slot is outside `table->slot_count` | assertion failure/abort | [x] |
| 12 | `stbds_hmdel_key` | decrement would make `used_count < 0` | assertion failure/abort | [x] |
| 13 | `stbds_hmdel_key` | moved final element cannot be found | assertion failure/abort | [x] |
| 14 | `stbds_hmdel_key` | moved element's slot does not point at `final_index` | assertion failure/abort | [x] |
| 15 | `stbds_stralloc` | after allocation, `len > a->remaining` | assertion failure/abort | [x] |
| 16 | `sh_geti` | lookup of `"foo"` in initial null map is not `-1` | assertion failure/abort | [x] |
| 17 | `sh_geti` | lookup of `"foo"` after selecting either string mode is not `-1` | assertion failure/abort | [x] |
| 18 | `sh_geti` | lookup of `"foo"` after setting default is not `-1` | assertion failure/abort | [x] |
| 19 | `sh_geti` | inserted even key does not map to `i*3`, or absent odd key does not map to `-2` | assertion failure/abort | [x] |
| 20 | `sh_geti` | after deleting keys `2 mod 4`, retained `0 mod 4` key is wrong or another key does not map to `-2` | assertion failure/abort | [x] |
| 21 | `sh_geti` | after deleting all keys, any lookup does not map to default `-2` | assertion failure/abort | [x] |

Rows 9-21 are internal consistency assertions. No well-formed public FFI input
can make their predicates false; differential tests exercise each assertion
site with the valid state transitions that lead to it. Tests do not fabricate
private allocator/hash metadata, because doing so is undefined behavior in C.

