# Configuration surface

Rows are derived from exported entry points plus branches on null/existing
state, size/capacity thresholds, hash input remainder length, binary/string
mode, string ownership mode, map load/tombstone thresholds, and arena block
sizes in `src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_rand_seed`, binary map APIs | explicit seeds; create fresh binary maps so the seed is consumed by `stbds_make_hash_index` | [x] |
| 2 | `stbds_rand_seed`, string map APIs | explicit seeds; create fresh string maps so the seed is consumed by `stbds_make_hash_index` | [x] |
| 3 | `stbds_hash_string` | empty NUL-terminated string; varied seeds | [x] |
| 4 | `stbds_hash_string` | non-empty ASCII strings, including one and many bytes; varied seeds | [x] |
| 5 | `stbds_hash_string` | strings containing high-bit non-NUL bytes; varied seeds | [x] |
| 6 | `stbds_hash_bytes` | length 0 with non-null pointer; varied seeds | [x] |
| 7 | `stbds_hash_bytes` | tail-only lengths 1 through 7 (every `switch` remainder case) | [x] |
| 8 | `stbds_hash_bytes` | exactly one full `sizeof(size_t)` block (8 bytes on this build) | [x] |
| 9 | `stbds_hash_bytes` | multiple full blocks with no remainder | [x] |
| 10 | `stbds_hash_bytes` | one or more full blocks plus each remainder 1 through 7, including high-bit bytes | [x] |
| 11 | `stbds_arrgrowf` | null array, zero requested length/capacity (no allocation) | [x] |
| 12 | `stbds_arrgrowf` | null array, `addlen > 0`, requested capacity below 4 (minimum capacity branch) | [x] |
| 13 | `stbds_arrgrowf` | null array, explicit `min_cap` 1 through 3 (minimum capacity branch) | [x] |
| 14 | `stbds_arrgrowf` | null array, explicit `min_cap >= 4` (exact requested capacity) | [x] |
| 15 | `stbds_arrgrowf` | existing array, requested minimum/added length fits current capacity (same pointer/no growth) | [x] |
| 16 | `stbds_arrgrowf` | existing array, required capacity is below twice current capacity (doubling branch) | [x] |
| 17 | `stbds_arrgrowf` | existing array, required capacity is at least twice current capacity (exact requirement branch) | [x] |
| 18 | `stbds_arrgrowf`, `stbds_arrfreef` | element sizes 1, 4, and 16; reallocation preserves header and element bytes; free non-null allocation | [x] |
| 19 | `stbds_hmget_key_ts` | null map lookup creates zeroed default entry and reports index `-1` | [x] |
| 20 | `stbds_hmget_key` | null map lookup creates default entry and stores header temp `-1` | [x] |
| 21 | `stbds_hmput_default` | null map creates one zeroed default entry | [x] |
| 22 | `stbds_hmput_default` | existing map/default entry is returned unchanged | [x] |
| 23 | binary map APIs, `stbds_shmode_func(mode=0)` | new insert, repeated-key update path, hit lookup, and absent lookup with fixed-size binary keys | [x] |
| 24 | binary map APIs | many inserts cross 8-slot load threshold and repeatedly double/rehash | [x] |
| 25 | binary map APIs, `stbds_hmdel_key` | delete first, middle, and final entries; non-final delete moves the last entry | [x] |
| 26 | binary map APIs, `stbds_hmdel_key` | enough insertions/deletions to exercise shrink and same-size tombstone rebuild branches | [x] |
| 27 | `stbds_hmfree_func` | free populated binary map | [x] |
| 28 | `stbds_shmode_func(mode=1)`, string map APIs | `STBDS_SH_DEFAULT`: borrowed key insertion, lookup, update, and free | [x] |
| 29 | `stbds_shmode_func(mode=2)`, string map APIs | `STBDS_SH_STRDUP`: duplicated key insertion, lookup, update, delete, and free | [x] |
| 30 | `stbds_shmode_func(mode=3)`, string map APIs | `STBDS_SH_ARENA`: arena-owned key insertion, lookup, update, delete, and free | [x] |
| 31 | `stbds_hmput_key(mode=1)` | null map implicitly selects `STBDS_SH_DEFAULT` string ownership | [x] |
| 32 | string map APIs | repeated string key finds existing entry and exposes stored key through temp-key state | [x] |
| 33 | string map APIs, `stbds_hmdel_key` | absent delete and present non-final delete with moved string-key index repair | [x] |
| 34 | `stbds_hmput_default`, get/delete APIs | allocated default-only map with no hash table: lookup miss and delete no-op branches | [x] |
| 35 | `stbds_stralloc` | empty/short string with empty arena allocates the minimum 512-byte block | [x] |
| 36 | `stbds_stralloc` | repeated short strings fit in the current block, including exact remaining-space boundary | [x] |
| 37 | `stbds_stralloc` | string larger than the current exponentially selected block takes oversized-block branch | [x] |
| 38 | `stbds_stralloc`, `stbds_strreset` | reset empty and populated arenas; all fields become zero | [x] |
| 39 | `strkey` | negative, zero, positive, and `int` boundary values formatted into shared 256-byte buffer | [x] |
| 40 | `arr_ins` | negative, zero, positive, and `int` boundary values across all five insertion indices | [x] |

## Feature combinations

`Cargo.toml` declares no features. The complete feature matrix is therefore:

1. default feature set (empty)
2. `--no-default-features` (also empty, verified separately)

