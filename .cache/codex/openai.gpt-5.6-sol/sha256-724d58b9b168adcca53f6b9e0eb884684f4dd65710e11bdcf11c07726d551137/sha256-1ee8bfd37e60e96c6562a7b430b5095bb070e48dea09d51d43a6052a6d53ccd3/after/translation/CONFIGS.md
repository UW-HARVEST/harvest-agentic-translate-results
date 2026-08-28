# Configuration Surface

The rows below come from the branch axes in `src/lib.c`: null/existing storage,
capacity transitions, byte-length remainder cases, binary/string comparison,
string ownership mode, table growth/rebuild/shrink, key hit/miss/update, and
arena block-size transitions. All 16 dynamic entry points are included.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array; `addlen = 0`, `min_cap = 0`; early return leaves pointer null | [x] |
| 2 | `stbds_arrgrowf` | null array; randomized element sizes and `addlen > min_cap` | [x] |
| 3 | `stbds_arrgrowf` | null array; randomized element sizes and `min_cap >= addlen` | [x] |
| 4 | `stbds_arrgrowf` | existing array; requested capacity already available, pointer/capacity unchanged | [x] |
| 5 | `stbds_arrgrowf` | existing array; request below twice the old capacity, capacity doubles | [x] |
| 6 | `stbds_arrgrowf` | existing array; request at/above twice the old capacity, requested capacity used | [x] |
| 7 | `stbds_arrfreef` | non-null arrays with element widths 1, 4, 8, and 16 | [x] |
| 8 | `stbds_rand_seed`, `stbds_hmput_key` | randomized seeds, then first binary table creation consumes that seed | [x] |
| 9 | `stbds_hash_bytes` | zero-byte input (`len % 8 == 0`, no full block) with randomized seeds | [x] |
| 10 | `stbds_hash_bytes` | tail lengths 1 through 7, covering every `switch (len-i)` case | [x] |
| 11 | `stbds_hash_bytes` | one or more full 8-byte blocks and remainder 0 | [x] |
| 12 | `stbds_hash_bytes` | one or more full blocks and each remainder 1 through 7 | [x] |
| 13 | `stbds_hash_string` | empty NUL-terminated string with randomized seeds | [x] |
| 14 | `stbds_hash_string` | nonempty ASCII strings of randomized length/value | [x] |
| 15 | `stbds_hash_string` | nonempty strings containing bytes with the high bit set | [x] |
| 16 | `stbds_hmput_default` | null map, creating one zeroed default element | [x] |
| 17 | `stbds_hmput_default` | existing map with a default element, no allocation or reset | [x] |
| 18 | `stbds_hmget_key_ts` | null binary map, producing default element and `temp = -1` | [x] |
| 19 | `stbds_hmget_key` | null binary map, writing `header.temp = -1` | [x] |
| 20 | `stbds_hmget_key`, `stbds_hmget_key_ts` | binary map with no hash table | [x] |
| 21 | `stbds_hmput_key` | binary mode; insert randomized 1-byte keys | [x] |
| 22 | `stbds_hmput_key` | binary mode; insert randomized 4-byte keys | [x] |
| 23 | `stbds_hmput_key` | binary mode; insert randomized 8-byte and composite keys | [x] |
| 24 | `stbds_hmput_key` | binary mode; update an existing key rather than append | [x] |
| 25 | `stbds_hmput_key` | binary mode; enough distinct keys to grow array and hash table repeatedly | [x] |
| 26 | `stbds_hmget_key`, `stbds_hmget_key_ts` | binary mode; existing-key lookup | [x] |
| 27 | `stbds_hmget_key`, `stbds_hmget_key_ts` | binary mode; absent-key lookup returning index `-1` | [x] |
| 28 | `stbds_hmdel_key` | binary mode; delete from null map | [x] |
| 29 | `stbds_hmdel_key` | binary mode; delete absent key from initialized table | [x] |
| 30 | `stbds_hmdel_key` | binary mode; delete final element | [x] |
| 31 | `stbds_hmdel_key` | binary mode; delete non-final element and repair moved index | [x] |
| 32 | `stbds_hmdel_key` | binary mode; tombstones exceed threshold and table rebuilds | [x] |
| 33 | `stbds_hmdel_key` | binary mode; used count crosses shrink threshold and table halves | [x] |
| 34 | `stbds_hmfree_func` | binary map after zero, one, and many entries | [x] |
| 35 | `stbds_shmode_func`, `stbds_hmput_key` | `STBDS_SH_DEFAULT`; string pointer is borrowed | [x] |
| 36 | `stbds_shmode_func`, `stbds_hmput_key` | `STBDS_SH_STRDUP`; string is duplicated | [x] |
| 37 | `stbds_shmode_func`, `stbds_hmput_key` | `STBDS_SH_ARENA`; string is arena allocated | [x] |
| 38 | `stbds_shmode_func`, `stbds_hmput_key` | mode `STBDS_SH_NONE`; binary key storage branch | [x] |
| 39 | `stbds_hmput_key` | string comparison mode with insert, existing-key update, hash-table growth, and collisions | [x] |
| 40 | `stbds_hmget_key`, `stbds_hmget_key_ts` | string mode; existing and absent string lookups | [x] |
| 41 | `stbds_hmdel_key` | string default/arena modes; delete existing and absent keys | [x] |
| 42 | `stbds_hmdel_key` | string strdup mode; delete frees owned key and repairs moved index | [x] |
| 43 | `stbds_hmfree_func` | each string ownership mode after zero, one, and many entries | [x] |
| 44 | `stbds_stralloc` | empty and short strings fitting in the initial 512-byte block | [x] |
| 45 | `stbds_stralloc` | repeated strings exhaust a block and advance block-size state | [x] |
| 46 | `stbds_stralloc` | string longer than current block, both with empty and existing arena storage | [x] |
| 47 | `stbds_stralloc` | block growth reaches the 1 MiB maximum branch | [x] |
| 48 | `stbds_strreset` | empty arena and arena containing one/many blocks | [x] |
| 49 | `strkey` | randomized negative, zero, and positive `int` values within the 256-byte buffer | [x] |
| 50 | `intput` | randomized values excluding collision values 9 and 11 | [x] |
