# Error surface

The C implementation has no error enum and no ordinary `-1`/`NULL` failure
return other than lookup misses and the explicit null-map deletion sentinel.
Lookup misses are valid map operations and are covered in `CONFIGS.md`.
The rows below include every explicit C assertion plus the generic invalid FFI
boundaries required by the verification protocol. “Process fault” records the
C implementation's actual unchecked-pointer behavior; it is not normalized
into a Rust error.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `stbds_arrfreef` | `a == NULL` | unchecked header subtraction/free; process faults | [x] |
| 2 | `stbds_hash_string` | `str == NULL` | unchecked dereference; process faults | [x] |
| 3 | `stbds_hash_bytes` | `p == NULL && len > 0` | unchecked byte read; process faults | [x] |
| 4 | `stbds_hmfree_func` | `a == NULL` | explicit no-op return | [x] |
| 5 | `stbds_hmget_key_ts` | `temp == NULL` | unchecked result write; process faults | [x] |
| 6 | `stbds_hmget_key` / `stbds_hmget_key_ts` | binary mode, populated table, `key == NULL && keysize > 0` | unchecked hash/key read; process faults | [x] |
| 7 | `stbds_hmget_key` / `stbds_hmget_key_ts` | string mode, populated table, `key == NULL` | unchecked string read; process faults | [x] |
| 8 | `stbds_hmput_key` | binary mode, `key == NULL && keysize > 0` | unchecked hash/key read; process faults | [x] |
| 9 | `stbds_hmput_key` | string mode, `key == NULL` | unchecked string read; process faults | [x] |
| 10 | `stbds_hmdel_key` | `a == NULL` | explicit `NULL` return | [x] |
| 11 | `stbds_hmdel_key` | binary mode, populated table, `key == NULL && keysize > 0` | unchecked hash/key read; process faults | [x] |
| 12 | `stbds_hmdel_key` | string mode, populated table, `key == NULL` | unchecked string read; process faults | [x] |
| 13 | `stbds_stralloc` | `a == NULL` | unchecked arena dereference; process faults | [x] |
| 14 | `stbds_stralloc` | `str == NULL` | unchecked `strlen`; process faults | [x] |
| 15 | `stbds_strreset` | `a == NULL` | unchecked arena dereference; process faults | [x] |
| 16 | `stbds_make_hash_index` (via map growth) | `used_count_threshold + tombstone_count_threshold >= slot_count` | `STBDS_ASSERT` aborts | [x] |
| 17 | `stbds_hmput_key` | post-growth invariant `(size_t)i + 1 > arrcap(a)` | `STBDS_ASSERT` aborts | [x] |
| 18 | `stbds_hmdel_key` | located `slot >= table->slot_count` | `STBDS_ASSERT` aborts | [x] |
| 19 | `stbds_hmdel_key` | deletion makes unsigned `table->used_count < 0` | assertion condition `used_count >= 0` is tautologically true for `size_t`; no rejection | [x] |
| 20 | `stbds_hmdel_key` | moved final element cannot be found in the hash index (`slot < 0`) | `STBDS_ASSERT` aborts | [x] |
| 21 | `stbds_hmdel_key` | moved element's bucket index is not `final_index` | `STBDS_ASSERT` aborts | [x] |
| 22 | `stbds_stralloc` | allocation branch leaves `len > a->remaining` | `STBDS_ASSERT` aborts | [x] |
| 23 | `sh_puts` | inserted arena key does not begin with byte `'a'` | `STBDS_ASSERT` aborts | [x] |
| 24 | `sh_puts` | arena-stored key aliases the source literal | `STBDS_ASSERT` aborts | [x] |
| 25 | `sh_puts` | stored value differs from input `num` | `STBDS_ASSERT` aborts | [x] |

The assertions in rows 17–25 are postconditions/internal consistency checks.
Their invalid states cannot be supplied as ordinary typed API arguments; the
differential suite exercises the corresponding operations and, where an
internal state can be corrupted through the exposed allocation layout, tests
the abort path in isolated child processes.
