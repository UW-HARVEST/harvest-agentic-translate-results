# Configuration Surface

Derived from all 16 dynamic entry points and every input-dependent `if`,
`switch`, threshold, and size remainder branch in `c_src/src/lib.c`. There are
no Cargo features, CMake options, or C preprocessor feature configurations.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array; `addlen` determines capacity; requested length below 4 | [x] |
| 2 | `stbds_arrgrowf` | null array; `min_cap` determines capacity and is at least 4 | [x] |
| 3 | `stbds_arrgrowf` | existing array; requested capacity does not exceed current capacity | [x] |
| 4 | `stbds_arrgrowf` | existing array; growth request is below twice current capacity, so capacity doubles | [x] |
| 5 | `stbds_arrgrowf` | existing array; growth request is at least twice current capacity | [x] |
| 6 | `stbds_arrgrowf`, `stbds_arrfreef` | element widths 1, 4, and a padded structure; allocate, preserve bytes over growth, free | [x] |
| 7 | `stbds_rand_seed` | seeds `0`, `1`, and full-width values before creating hash tables | [x] |
| 8 | `stbds_hash_string` | empty string with varied seeds | [x] |
| 9 | `stbds_hash_string` | nonempty strings, including bytes with the high bit set, with varied lengths/seeds | [x] |
| 10 | `stbds_hash_bytes` | length remainder 0 with no complete word (empty input) | [x] |
| 11 | `stbds_hash_bytes` | length remainder 1 with no complete word | [x] |
| 12 | `stbds_hash_bytes` | length remainder 2 with no complete word | [x] |
| 13 | `stbds_hash_bytes` | length remainder 3 with no complete word | [x] |
| 14 | `stbds_hash_bytes` | length remainder 4 with no complete word | [x] |
| 15 | `stbds_hash_bytes` | length remainder 5 with no complete word | [x] |
| 16 | `stbds_hash_bytes` | length remainder 6 with no complete word | [x] |
| 17 | `stbds_hash_bytes` | length remainder 7 with no complete word | [x] |
| 18 | `stbds_hash_bytes` | one or many complete `size_t` words plus each 0 through 7-byte remainder; randomized high-bit bytes/seeds | [x] |
| 19 | `stbds_hmput_default` | null map creates one zeroed default entry | [x] |
| 20 | `stbds_hmput_default` | existing default-only map is returned unchanged | [x] |
| 21 | `stbds_hmget_key`, `stbds_hmget_key_ts` | null map; binary mode (`mode < 1`) | [x] |
| 22 | `stbds_hmget_key`, `stbds_hmget_key_ts` | default-only map without a hash table | [x] |
| 23 | `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts` | binary modes (`mode < 1`), new key versus replacement, hit versus miss, varied key widths | [x] |
| 24 | `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts` | string modes (`mode >= 1`) with default borrowed-string storage, new key versus replacement, hit versus miss | [x] |
| 25 | `stbds_hmput_key` | enough unique keys to cross the 75% used threshold and repeatedly double the table | [x] |
| 26 | `stbds_hmput_key` | insertion reuses a deleted tombstone | [x] |
| 27 | `stbds_shmode_func`, `stbds_hmput_key` | mode `STBDS_SH_NONE` (`0`): binary key bytes are copied | [x] |
| 28 | `stbds_shmode_func`, `stbds_hmput_key` | mode `STBDS_SH_DEFAULT` (`1`): string pointer is borrowed | [x] |
| 29 | `stbds_shmode_func`, `stbds_hmput_key` | mode `STBDS_SH_STRDUP` (`2`): string is duplicated | [x] |
| 30 | `stbds_shmode_func`, `stbds_hmput_key` | mode `STBDS_SH_ARENA` (`3`): string is arena allocated | [x] |
| 31 | `stbds_shmode_func`, `stbds_hmput_key` | out-of-range mode values (negative, `4`, `255`, and values truncating to an in-range `unsigned char`) | [x] |
| 32 | `stbds_hmdel_key` | null map, default-only map, and populated map with absent key | [x] |
| 33 | `stbds_hmdel_key` | delete last element versus non-last element requiring moved-index repair; binary and string keys | [x] |
| 34 | `stbds_hmdel_key` | deletions cross the 25% shrink threshold on a table larger than 8 slots | [x] |
| 35 | `stbds_hmdel_key` | tombstones cross the 3/16 rebuild threshold without crossing shrink threshold | [x] |
| 36 | `stbds_hmdel_key` | `STBDS_HM_STRING` plus `STBDS_SH_STRDUP` frees the deleted owned key; other mode combinations do not | [x] |
| 37 | `stbds_hmfree_func` | null pointer; binary map; borrowed-string map; strdup map; arena map | [x] |
| 38 | `stbds_stralloc` | empty and short strings fit in a newly allocated 512-byte standard block | [x] |
| 39 | `stbds_stralloc` | repeated strings fit in remaining current-block space | [x] |
| 40 | `stbds_stralloc` | repeated exhaustion grows standard blocks through the capped 1 MiB block size | [x] |
| 41 | `stbds_stralloc` | string longer than current block size takes dedicated-block path with no prior storage | [x] |
| 42 | `stbds_stralloc` | string longer than current block size takes dedicated-block path with prior storage | [x] |
| 43 | `stbds_strreset` | empty arena and arena containing standard plus dedicated blocks | [x] |
| 44 | `strkey` | negative, zero, positive, and decimal-boundary `int` values; repeated call overwrites static buffer | [x] |
| 45 | `arr_ins` | negative, zero, positive, `INT_MIN`, and `INT_MAX` insertion values over all five insertion positions | [x] |
