# Configuration Surface

There are no Cargo features. The only build configuration is the empty/default
feature set. Runtime axes below come from the C branches, constants, mode
switches, and public entry points.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array, `addlen=0`, `min_cap=0`: no allocation | [x] |
| 2 | `stbds_arrgrowf`, `stbds_arrfreef` | null array, requested capacity `1..3`: floor capacity 4; element widths 1/4/16 | [x] |
| 3 | `stbds_arrgrowf`, `stbds_arrfreef` | null array, `addlen > min_cap`, capacity comes from add length | [x] |
| 4 | `stbds_arrgrowf` | existing array, requested capacity at or below capacity: pointer/capacity unchanged | [x] |
| 5 | `stbds_arrgrowf` | existing array, request above capacity but below double: capacity doubles | [x] |
| 6 | `stbds_arrgrowf` | existing array, request at/above double: explicit capacity wins and bytes survive realloc | [x] |
| 7 | `stbds_hash_bytes` | null/empty bytes, seeds 0 and nonzero | [x] |
| 8 | `stbds_hash_bytes` | tail length 1 | [x] |
| 9 | `stbds_hash_bytes` | tail length 2 | [x] |
| 10 | `stbds_hash_bytes` | tail length 3 | [x] |
| 11 | `stbds_hash_bytes` | tail length 4 (signed 32-bit promotion branch) | [x] |
| 12 | `stbds_hash_bytes` | tail length 5 | [x] |
| 13 | `stbds_hash_bytes` | tail length 6 | [x] |
| 14 | `stbds_hash_bytes` | tail length 7 | [x] |
| 15 | `stbds_hash_bytes` | exactly one full `size_t` block | [x] |
| 16 | `stbds_hash_bytes` | multiple full blocks plus tails 0..7 | [x] |
| 17 | `stbds_hash_string` | empty NUL-terminated string, seeds 0/nonzero | [x] |
| 18 | `stbds_hash_string` | nonempty ASCII and high-bit bytes, varied lengths/seeds | [x] |
| 19 | `stbds_rand_seed`, map APIs | same seed and operation sequence yields matching table behavior across libraries | [x] |
| 20 | `stbds_stralloc` | zeroed arena, empty/small first string (`len <= 512`) | [x] |
| 21 | `stbds_stralloc` | repeated small strings: fit current block, then allocate progressively larger blocks | [x] |
| 22 | `stbds_stralloc` | string length just above current block size: dedicated block, with empty arena | [x] |
| 23 | `stbds_stralloc` | dedicated oversized block while arena already has normal storage | [x] |
| 24 | `stbds_stralloc` | block growth reaches `1<<20` maximum and stops incrementing mode counter | [x] |
| 25 | `stbds_strreset` | empty and populated arenas: all fields zero afterward | [x] |
| 26 | `stbds_hmput_default` | null map and existing initialized map | [x] |
| 27 | `stbds_hmget_key_ts`, `stbds_hmget_key` | null map and map with no hash table | [x] |
| 28 | `stbds_hmput_key`, getters | binary mode 0, key widths 1/4/8/16, insert one/new key | [x] |
| 29 | `stbds_hmput_key`, getters | binary mode, duplicate key update path and missing lookup | [x] |
| 30 | `stbds_hmput_key`, getters | binary mode, many randomized keys crossing 8-slot growth thresholds | [x] |
| 31 | `stbds_hmdel_key` | binary mode: null map, no-table map, and missing key | [x] |
| 32 | `stbds_hmdel_key` | binary mode: delete final and non-final entries (move-last branch) | [x] |
| 33 | `stbds_hmdel_key` | binary mode: enough deletes for tombstone rebuild | [x] |
| 34 | `stbds_hmdel_key` | binary mode: enough inserts/deletes for table shrink | [x] |
| 35 | `stbds_shmode_func`, map APIs | `STBDS_SH_NONE` (0), binary key storage switch arm | [x] |
| 36 | `stbds_shmode_func`, map APIs | `STBDS_SH_DEFAULT` (1), borrowed string keys; empty/one/many/duplicate | [x] |
| 37 | `stbds_shmode_func`, map APIs | `STBDS_SH_STRDUP` (2), copied string keys; empty/one/many/duplicate | [x] |
| 38 | `stbds_shmode_func`, map APIs | `STBDS_SH_ARENA` (3), arena-copied keys; empty/one/many/duplicate | [x] |
| 39 | string map APIs | string lookup present/missing and deletion final/non-final under modes 1/2/3 | [x] |
| 40 | `stbds_hmfree_func` | null, binary, default-string, strdup-string, and arena-string maps | [x] |
| 41 | `strkey` | negative, zero, positive, and large `int` values | [x] |
| 42 | `str_dups` | negative, zero, one, and many iterations; exact stdout bytes | [x] |
| 43 | all exports | fixed-seed randomized operation/value sequences | [x] |
