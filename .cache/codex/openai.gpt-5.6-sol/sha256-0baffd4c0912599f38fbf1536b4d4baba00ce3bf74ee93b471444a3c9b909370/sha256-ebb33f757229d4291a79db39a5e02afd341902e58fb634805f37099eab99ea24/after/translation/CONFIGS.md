# Configuration surface

Rows are derived from public dynamic entry points and the `if`/`switch`
branches, constants, modes, and shape boundaries in `src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_rand_seed` | seed values 0, 1, ordinary, and `SIZE_MAX`; verify their effect on newly created hash tables | [x] |
| 2 | `stbds_hash_string` | empty NUL-terminated string; seeds 0/ordinary/`SIZE_MAX` | [x] |
| 3 | `stbds_hash_string` | one-byte and multi-byte ASCII strings | [x] |
| 4 | `stbds_hash_string` | bytes with the high bit set before the NUL terminator | [x] |
| 5 | `stbds_hash_bytes` | length 0 with null and non-null data pointers | [x] |
| 6 | `stbds_hash_bytes` | tail-only lengths 1 through 7, exercising every switch case | [x] |
| 7 | `stbds_hash_bytes` | exactly one 8-byte word | [x] |
| 8 | `stbds_hash_bytes` | multiple words, with each possible 0-through-7-byte tail | [x] |
| 9 | `stbds_arrgrowf`, `stbds_arrfreef` | null array; element sizes 1, 4, and structure-sized; `addlen=0`, `min_cap=0..4` minimum-capacity floor | [x] |
| 10 | `stbds_arrgrowf`, `stbds_arrfreef` | existing allocation where requested capacity is already sufficient (pointer/data unchanged) | [x] |
| 11 | `stbds_arrgrowf`, `stbds_arrfreef` | existing allocation where `addlen` forces growth and capacity doubles | [x] |
| 12 | `stbds_arrgrowf`, `stbds_arrfreef` | existing allocation where explicit `min_cap` exceeds both length and doubled capacity | [x] |
| 13 | `stbds_hmget_key_ts`, `stbds_hmget_key` | null map, binary mode 0: create zero default element and report index -1 | [x] |
| 14 | `stbds_hmget_key_ts`, `stbds_hmget_key` | map with default element but no hash table: miss/index -1 | [x] |
| 15 | `stbds_hmput_default` | null map versus already initialized map; zeroed default and idempotent second call | [x] |
| 16 | `stbds_hmput_key`, `stbds_hmget_key_ts`, `stbds_hmget_key` | binary keys, one insert then hit/update of existing key | [x] |
| 17 | same hash-map entry points | binary key sizes 1, 4, 8, and 16 bytes; randomized key bytes | [x] |
| 18 | same hash-map entry points | binary map crossing the 8-slot 75% threshold and rehashing to larger tables | [x] |
| 19 | same hash-map entry points | mode values less than 1 (including negative/out-of-range enum integers), which select binary hashing | [x] |
| 20 | same hash-map entry points | mode values greater than or equal to 1, which select string hashing/equality | [x] |
| 21 | `stbds_hmdel_key` | null map returns null; initialized default-only map returns unchanged | [x] |
| 22 | `stbds_hmdel_key` | binary map missing-key deletion leaves map unchanged and deletion temp=0 | [x] |
| 23 | `stbds_hmdel_key` | delete final binary element versus non-final element (move-last repair) | [x] |
| 24 | `stbds_hmdel_key` | repeated binary deletions trigger tombstone rebuild | [x] |
| 25 | `stbds_hmdel_key` | large binary map deletions cross shrink threshold and halve slot count | [x] |
| 26 | `stbds_hmput_key`, string hash APIs | null string map creates `STBDS_SH_DEFAULT` (1): borrowed string key storage | [x] |
| 27 | same | `STBDS_SH_STRDUP` (2): duplicate string key storage, update, delete, and free | [x] |
| 28 | same | `STBDS_SH_ARENA` (3): arena-owned string key storage, update, delete, and free | [x] |
| 29 | `stbds_shmode_func`, binary hash APIs | stored modes 0 and out-of-range integers: low byte stored and the default switch branch copies binary key bytes | [x] |
| 30 | `stbds_hmfree_func` | null map; default-only map; binary table; each string ownership mode | [x] |
| 31 | `stbds_stralloc` | empty and short strings fitting in the initial 512-byte block | [x] |
| 32 | `stbds_stralloc` | repeated strings exhaust a block and allocate the next geometric block | [x] |
| 33 | `stbds_stralloc` | lengths exactly at and one past the current block-size boundary | [x] |
| 34 | `stbds_stralloc` | oversized string above 1 MiB gets a dedicated block | [x] |
| 35 | `stbds_strreset` | empty arena and arena containing ordinary plus dedicated blocks; all fields zeroed | [x] |
| 36 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` formatting | [x] |
| 37 | `arr_push` | `num <= 0`, `1..50`, exactly 50, 51, and multiple outer-loop iterations | [x] |

Feature combinations: Cargo.toml declares no features, so the only build
configuration is the default/no-feature configuration.
