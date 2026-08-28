# Configuration surface

Constants and axes are mechanically derived from `src/lib.c`: hash mode
(`0` binary versus `>=1` string), string ownership mode (`0/1` default,
`2` strdup, `3` arena, other), 8-slot hash buckets and their grow/shrink
thresholds, SipHash tail length, arena block limits (512 and 1 MiB), null versus
existing state, and empty/one/many shapes. There are no Cargo features.

| # | entry point(s) | configuration (options set + input shape) | Verified |
|---|----------------|-------------------------------------------|----------|
| 1 | `stbds_arrgrowf` | null array, zero requested capacity returns null before minimum-4 branch | [x] |
| 2 | `stbds_arrgrowf` | null array, positive `addlen` supplies minimum length (minimum 4 or exact larger request) | [x] |
| 3 | `stbds_arrgrowf` | existing array, requested capacity already available | [x] |
| 4 | `stbds_arrgrowf` | existing array, growth selects doubled capacity | [x] |
| 5 | `stbds_arrgrowf` | existing array, explicit minimum exceeds doubled capacity | [x] |
| 6 | `stbds_arrfreef` | free a successfully allocated dynamic array | [x] |
| 7 | `stbds_rand_seed` | seed zero before creating a map | [x] |
| 8 | `stbds_rand_seed` | nonzero/random seed before creating a map | [x] |
| 9 | `stbds_hash_string` | empty NUL-terminated byte string | [x] |
| 10 | `stbds_hash_string` | one-byte string | [x] |
| 11 | `stbds_hash_string` | many-byte ASCII string | [x] |
| 12 | `stbds_hash_string` | bytes with the high bit set before NUL | [x] |
| 13 | `stbds_hash_bytes` | length remainder 0 with empty input | [x] |
| 14 | `stbds_hash_bytes` | length remainder 1 | [x] |
| 15 | `stbds_hash_bytes` | length remainder 2 | [x] |
| 16 | `stbds_hash_bytes` | length remainder 3 | [x] |
| 17 | `stbds_hash_bytes` | length remainder 4 | [x] |
| 18 | `stbds_hash_bytes` | length remainder 5 | [x] |
| 19 | `stbds_hash_bytes` | length remainder 6 | [x] |
| 20 | `stbds_hash_bytes` | length remainder 7 | [x] |
| 21 | `stbds_hash_bytes` | exactly one full `size_t` block | [x] |
| 22 | `stbds_hash_bytes` | multiple full blocks plus each tail remainder | [x] |
| 23 | `stbds_hmfree_func` | binary map with entries | [x] |
| 24 | `stbds_hmfree_func` | default string map with borrowed keys | [x] |
| 25 | `stbds_hmfree_func` | strdup string map with owned keys | [x] |
| 26 | `stbds_hmfree_func` | arena string map with arena-owned keys | [x] |
| 27 | `stbds_hmget_key_ts` | null map creates zero default entry and returns `temp=-1` | [x] |
| 28 | `stbds_hmget_key_ts` | initialized map with no hash table | [x] |
| 29 | `stbds_hmget_key_ts` | hash table present, absent key | [x] |
| 30 | `stbds_hmget_key_ts` | hash table present, existing binary key | [x] |
| 31 | `stbds_hmget_key_ts` | hash table present, existing string key | [x] |
| 32 | `stbds_hmget_key` | absent key writes `-1` to header temp | [x] |
| 33 | `stbds_hmget_key` | existing binary key writes its index to header temp | [x] |
| 34 | `stbds_hmget_key` | existing string key writes its index to header temp | [x] |
| 35 | `stbds_hmput_default` | null map creates one zeroed default record | [x] |
| 36 | `stbds_hmput_default` | existing zero-length raw map creates default record | [x] |
| 37 | `stbds_hmput_default` | existing map with default record is unchanged | [x] |
| 38 | `stbds_hmput_key` | binary mode, first insertion into null map | [x] |
| 39 | `stbds_hmput_key` | binary mode, update existing key | [x] |
| 40 | `stbds_hmput_key` | binary mode, many randomized keys crossing 8-slot growth threshold | [x] |
| 41 | `stbds_hmput_key` | string mode with default borrowed-key storage | [x] |
| 42 | `stbds_hmput_key` | string mode with strdup storage | [x] |
| 43 | `stbds_hmput_key` | string mode with arena storage | [x] |
| 44 | `stbds_hmput_key` | string mode, update an existing key | [x] |
| 45 | `stbds_hmput_key` | insertion reuses a deleted tombstone | [x] |
| 46 | `stbds_hmput_key` | out-of-range negative mode follows binary branch | [x] |
| 47 | `stbds_hmput_key` | out-of-range positive mode follows string branch | [x] |
| 48 | `stbds_shmode_func` | `STBDS_SH_NONE` (`0`) | [x] |
| 49 | `stbds_shmode_func` | `STBDS_SH_DEFAULT` (`1`) | [x] |
| 50 | `stbds_shmode_func` | `STBDS_SH_STRDUP` (`2`) | [x] |
| 51 | `stbds_shmode_func` | `STBDS_SH_ARENA` (`3`) | [x] |
| 52 | `stbds_shmode_func` | out-of-range ownership mode | [x] |
| 53 | `stbds_hmdel_key` | null map | [x] |
| 54 | `stbds_hmdel_key` | initialized map without hash table | [x] |
| 55 | `stbds_hmdel_key` | missing key in populated map | [x] |
| 56 | `stbds_hmdel_key` | delete sole/last binary entry | [x] |
| 57 | `stbds_hmdel_key` | delete non-final binary entry and move final entry | [x] |
| 58 | `stbds_hmdel_key` | delete strdup string key and free owned key | [x] |
| 59 | `stbds_hmdel_key` | enough deletions to shrink a grown table | [x] |
| 60 | `stbds_hmdel_key` | enough deletions to rebuild after tombstones | [x] |
| 61 | `stbds_hmdel_key` | out-of-range negative/positive modes take binary/string comparisons | [x] |
| 62 | `stbds_stralloc` | empty string in zeroed arena allocates initial 512-byte block | [x] |
| 63 | `stbds_stralloc` | randomized small strings fit in current block | [x] |
| 64 | `stbds_stralloc` | strings exhaust blocks and advance block growth state | [x] |
| 65 | `stbds_stralloc` | string larger than current block gets a dedicated block | [x] |
| 66 | `stbds_stralloc` | string larger than 1 MiB maximum gets a dedicated block | [x] |
| 67 | `stbds_strreset` | zeroed/empty arena | [x] |
| 68 | `stbds_strreset` | populated arena with regular and dedicated blocks | [x] |
| 69 | `strkey` | negative integer | [x] |
| 70 | `strkey` | zero | [x] |
| 71 | `strkey` | positive integer | [x] |
| 72 | `strkey` | `INT_MIN` and `INT_MAX` boundaries | [x] |
| 73 | `str_put` | negative/zero count (empty arena loop) | [x] |
| 74 | `str_put` | count one | [x] |
| 75 | `str_put` | randomized small positive counts | [x] |
| 76 | `str_put` | count large enough to allocate multiple arena blocks | [x] |
