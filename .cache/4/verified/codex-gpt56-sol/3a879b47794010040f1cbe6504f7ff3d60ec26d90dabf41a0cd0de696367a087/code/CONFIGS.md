# Configuration Surface

The crate and CMake project define no build-time options. There is one build
configuration: Cargo with no features and CMake with no definitions.

Rows below are the pruned cross-product of public entry points with the runtime
branches, modes, state transitions, and input shapes in `c_src/src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array, `addlen = 0`, `min_cap = 0` (no allocation) | [x] |
| 2 | `stbds_arrgrowf` | null array, requested capacity `1..3` (minimum capacity becomes 4) | [x] |
| 3 | `stbds_arrgrowf` | null array, `addlen > min_cap` (length requirement wins) | [x] |
| 4 | `stbds_arrgrowf` | existing array, request at or below capacity (same pointer/no growth) | [x] |
| 5 | `stbds_arrgrowf` | existing array, request above capacity but below twice capacity (doubling branch) | [x] |
| 6 | `stbds_arrgrowf` | existing array, request at or above twice capacity (requested capacity branch) | [x] |
| 7 | `stbds_arrfreef` | free a non-null allocation returned by `stbds_arrgrowf` | [x] |
| 8 | `stbds_rand_seed` + map creation | seed values `0`, ordinary, and `SIZE_MAX`; observe seed through deterministic map operations | [x] |
| 9 | `stbds_hash_string` | empty NUL-terminated string across randomized seeds | [x] |
| 10 | `stbds_hash_string` | one-byte string, including bytes `1..255`, across randomized seeds | [x] |
| 11 | `stbds_hash_string` | multi-byte strings across randomized lengths, bytes, and seeds | [x] |
| 12 | `stbds_hash_bytes` | length remainder 0 with no full word (`len = 0`, including null pointer) | [x] |
| 13 | `stbds_hash_bytes` | tail length 1 | [x] |
| 14 | `stbds_hash_bytes` | tail length 2 | [x] |
| 15 | `stbds_hash_bytes` | tail length 3 | [x] |
| 16 | `stbds_hash_bytes` | tail length 4, including high-bit byte 3 | [x] |
| 17 | `stbds_hash_bytes` | tail length 5 | [x] |
| 18 | `stbds_hash_bytes` | tail length 6 | [x] |
| 19 | `stbds_hash_bytes` | tail length 7 | [x] |
| 20 | `stbds_hash_bytes` | one full `size_t` word (`len = 8`) | [x] |
| 21 | `stbds_hash_bytes` | multiple full words plus each possible tail (`len > 8`) | [x] |
| 22 | `stbds_hmfree_func` | null map (no-op) | [x] |
| 23 | `stbds_hmfree_func` | binary map with entries | [x] |
| 24 | `stbds_hmfree_func` | default-pointer, strdup, and arena string maps with entries | [x] |
| 25 | `stbds_hmget_key_ts` | null map creates only the zeroed default entry | [x] |
| 26 | `stbds_hmget_key_ts` | map with default entry but no hash table | [x] |
| 27 | `stbds_hmget_key_ts` | binary table hit and miss, caller-provided `temp` | [x] |
| 28 | `stbds_hmget_key_ts` | string table hit and miss, caller-provided `temp` | [x] |
| 29 | `stbds_hmget_key` | binary/string hit and miss, result stored in header `temp` | [x] |
| 30 | `stbds_hmput_default` | null map creates a zeroed default element | [x] |
| 31 | `stbds_hmput_default` | existing zero-length raw map creates default element | [x] |
| 32 | `stbds_hmput_default` | existing map already has default element (no-op) | [x] |
| 33 | `stbds_hmput_key` | binary mode 0, first insertion into null map | [x] |
| 34 | `stbds_hmput_key` | binary mode 0, update an existing key | [x] |
| 35 | `stbds_hmput_key` | binary mode 0, many unique keys causing array and table growth/rehash | [x] |
| 36 | `stbds_hmput_key` | binary mode 0, insert after deletion and reuse a tombstone | [x] |
| 37 | `stbds_hmput_key` | binary mode `< 1` outside documented range (`mode = -1`) | [x] |
| 38 | `stbds_hmput_key` | string mode default-pointer (`mode = 1`), first/unique insertion | [x] |
| 39 | `stbds_hmput_key` | string mode default-pointer, duplicate textual key from a different pointer | [x] |
| 40 | `stbds_hmput_key` | string mode `>= 1` outside documented range (`mode = 2`) on an implicit map | [x] |
| 41 | `stbds_shmode_func` | explicit `STBDS_SH_DEFAULT` mode (1), then insert/get/delete | [x] |
| 42 | `stbds_shmode_func` | explicit `STBDS_SH_STRDUP` mode (2), then insert/get/delete | [x] |
| 43 | `stbds_shmode_func` | explicit `STBDS_SH_ARENA` mode (3), then insert/get/delete | [x] |
| 44 | `stbds_shmode_func` | mode 0 and out-of-range modes, including `-1` truncation to `unsigned char` | [x] |
| 45 | `stbds_hmdel_key` | null map | [x] |
| 46 | `stbds_hmdel_key` | map with default entry but no table | [x] |
| 47 | `stbds_hmdel_key` | table miss | [x] |
| 48 | `stbds_hmdel_key` | delete last element (no compaction) | [x] |
| 49 | `stbds_hmdel_key` | delete non-last binary element (move and repair index) | [x] |
| 50 | `stbds_hmdel_key` | delete non-last string element (move and repair index) | [x] |
| 51 | `stbds_hmdel_key` | enough deletions to shrink a table | [x] |
| 52 | `stbds_hmdel_key` | enough tombstones to rebuild without shrinking | [x] |
| 53 | `stbds_stralloc` | empty and short strings allocated into a new normal 512-byte block | [x] |
| 54 | `stbds_stralloc` | repeated strings fit in remaining current block | [x] |
| 55 | `stbds_stralloc` | block growth sequence up to the 1 MiB cap | [x] |
| 56 | `stbds_stralloc` | string larger than current block gets a dedicated block, with arena empty/populated | [x] |
| 57 | `stbds_strreset` | empty arena and populated arena | [x] |
| 58 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` values | [x] |
| 59 | `str_put` | `num <= 0` (arena loop skipped) | [x] |
| 60 | `str_put` | small positive `num` (arena loop executes in one block) | [x] |
| 61 | `str_put` | larger positive `num` (arena allocates multiple blocks) | [x] |
