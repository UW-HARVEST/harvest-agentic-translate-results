# Configuration Surface

There are no Cargo features and no C `#ifdef` configuration branches. The
rows below are the runtime cross-product pruned to branch-distinct cases in
the C implementation.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `stbds_arrgrowf` | Null array; `addlen = 0`, `min_cap = 0` (no allocation) | [x] |
| 2 | `stbds_arrgrowf`, `stbds_arrfreef` | Null array; requested capacity `1..3` (minimum capacity 4); element sizes 1, 4, and 16 | [x] |
| 3 | `stbds_arrgrowf`, `stbds_arrfreef` | Null array; `addlen > min_cap`; capacity selected from resulting length | [x] |
| 4 | `stbds_arrgrowf`, `stbds_arrfreef` | Existing array; request at or below capacity returns unchanged allocation/data | [x] |
| 5 | `stbds_arrgrowf`, `stbds_arrfreef` | Existing array; requested capacity below twice old capacity (doubling branch) | [x] |
| 6 | `stbds_arrgrowf`, `stbds_arrfreef` | Existing array; requested capacity at least twice old capacity (explicit-capacity branch) | [x] |
| 7 | `stbds_hash_bytes` | Empty bytes (`len = 0`), both null and non-null pointers, randomized seeds | [x] |
| 8 | `stbds_hash_bytes` | Tail lengths 1 through 7, including bytes with the high bit set | [x] |
| 9 | `stbds_hash_bytes` | One full `size_t` word (`len = 8`) | [x] |
| 10 | `stbds_hash_bytes` | Multiple full words plus every tail length 0 through 7 | [x] |
| 11 | `stbds_hash_string` | Empty NUL-terminated string and randomized seeds | [x] |
| 12 | `stbds_hash_string` | Non-empty strings, including high-bit bytes before NUL | [x] |
| 13 | `stbds_rand_seed`, `stbds_hmput_key` | Seed set before first map allocation; binary and string maps | [x] |
| 14 | `stbds_stralloc`, `stbds_strreset` | Empty/short string into an empty arena (`len <= 512`) | [x] |
| 15 | `stbds_stralloc`, `stbds_strreset` | Repeated strings that fit the current arena block | [x] |
| 16 | `stbds_stralloc`, `stbds_strreset` | Repeated strings crossing a block boundary and advancing the block-size schedule | [x] |
| 17 | `stbds_stralloc`, `stbds_strreset` | String larger than the selected block size (dedicated block branch) | [x] |
| 18 | `stbds_strreset` | Zero-initialized empty arena and populated arena | [x] |
| 19 | `stbds_hmput_default`, `stbds_hmfree_func` | Null map creates one zero default element; repeated call is a no-op | [x] |
| 20 | `stbds_hmget_key_ts`, `stbds_hmget_key`, `stbds_hmfree_func` | Null map and map with default element but no hash table | [x] |
| 21 | `stbds_hmput_key`, `stbds_hmget_key_ts`, `stbds_hmget_key` | Binary keys; new insert, existing-key update path, present and absent lookup | [x] |
| 22 | `stbds_hmput_key`, `stbds_hmget_key_ts`, `stbds_hmget_key` | Binary key sizes 1, 4, 8, and 16 with correspondingly sized records | [x] |
| 23 | `stbds_hmput_key`, `stbds_hmget_key*` | Binary map crossing 75% load threshold, rehashing from 8 to 16+ slots | [x] |
| 24 | `stbds_hmput_key`, `stbds_hmget_key*` | Colliding/probed binary keys, including wrapped bucket scans | [x] |
| 25 | `stbds_hmput_key`, `stbds_hmget_key*` | String mode (`mode = 1`), empty and non-empty keys, borrowed/default storage | [x] |
| 26 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key*` | String storage mode `STBDS_SH_STRDUP` (`2`) | [x] |
| 27 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key*` | String storage mode `STBDS_SH_ARENA` (`3`), short and dedicated-block keys | [x] |
| 28 | `stbds_hmdel_key` | Null map, map without table, and absent binary/string key | [x] |
| 29 | `stbds_hmdel_key` | Delete present final element vs non-final element (move-last branch), binary/string | [x] |
| 30 | `stbds_hmdel_key` | Deletions crossing tombstone rebuild threshold | [x] |
| 31 | `stbds_hmdel_key` | Deletions crossing used-count shrink threshold after prior growth | [x] |
| 32 | `stbds_hmfree_func` | Null, binary, borrowed-string, strdup-string, and arena-string maps | [x] |
| 33 | `stbds_hmput_key`, `stbds_hmget_key*`, `stbds_hmdel_key` | Out-of-range low mode (`-1`) as binary and out-of-range high mode (`2`) as string | [x] |
| 34 | `stbds_shmode_func` | Modes 0, 1, 2, 3 and out-of-range values, with allocation/free lifecycle | [x] |
| 35 | `strkey` | Negative, zero, and positive `int`, including decimal-width boundaries | [x] |
| 36 | `arr_push` | `num <= 0`, `1..50`, exactly 50, and multiple 50-step iterations | [x] |
| 37 | `stbds_stralloc`, `stbds_strreset` | Repeated dedicated blocks advance the schedule to `STBDS_STRING_ARENA_BLOCKSIZE_MAX` (1 MiB), where `block` stops incrementing | [x] |
