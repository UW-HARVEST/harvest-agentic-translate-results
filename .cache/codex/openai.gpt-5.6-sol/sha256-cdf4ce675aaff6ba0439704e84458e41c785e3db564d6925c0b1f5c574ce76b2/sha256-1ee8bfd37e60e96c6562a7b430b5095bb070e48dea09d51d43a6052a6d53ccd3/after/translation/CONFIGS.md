# Configuration surface

There are no Cargo features or C compile-time feature switches. The rows below
come from the public exports and the runtime branches in `lib.c`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|---|
| 1 | `stbds_rand_seed` | seed `0`, nonzero, and `SIZE_MAX`; subsequent fresh table creation | [x] |
| 2 | `stbds_hash_bytes` | empty input (`len = 0`, null pointer allowed because it is not read) | [x] |
| 3 | `stbds_hash_bytes` | no full word and tail remainder `len % 8 = 1` | [x] |
| 4 | `stbds_hash_bytes` | no full word and tail remainder `len % 8 = 2` | [x] |
| 5 | `stbds_hash_bytes` | no full word and tail remainder `len % 8 = 3` | [x] |
| 6 | `stbds_hash_bytes` | no full word and tail remainder `len % 8 = 4` | [x] |
| 7 | `stbds_hash_bytes` | no full word and tail remainder `len % 8 = 5` | [x] |
| 8 | `stbds_hash_bytes` | no full word and tail remainder `len % 8 = 6` | [x] |
| 9 | `stbds_hash_bytes` | no full word and tail remainder `len % 8 = 7` | [x] |
| 10 | `stbds_hash_bytes` | one full 8-byte word, no tail | [x] |
| 11 | `stbds_hash_bytes` | multiple full words with each tail remainder `0..7` | [x] |
| 12 | `stbds_hash_string` | empty NUL-terminated string | [x] |
| 13 | `stbds_hash_string` | one-byte string | [x] |
| 14 | `stbds_hash_string` | multi-byte ASCII and bytes `>= 0x80` before NUL | [x] |
| 15 | `stbds_arrgrowf`, `stbds_arrfreef` | null array, `addlen = 0`, `min_cap = 0..3`; minimum capacity branch | [x] |
| 16 | `stbds_arrgrowf`, `stbds_arrfreef` | null array, `addlen > min_cap`; minimum length wins | [x] |
| 17 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array, requested capacity already available; pointer unchanged | [x] |
| 18 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array, request below twice capacity; doubling branch and bytes preserved | [x] |
| 19 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array, explicit large `min_cap`; requested capacity branch and bytes preserved | [x] |
| 20 | `stbds_stralloc`, `stbds_strreset` | zeroed arena, empty/short string; allocate 512-byte normal block | [x] |
| 21 | `stbds_stralloc`, `stbds_strreset` | repeated strings that fit current block | [x] |
| 22 | `stbds_stralloc`, `stbds_strreset` | string exhausts block and causes geometrically larger normal block | [x] |
| 23 | `stbds_stralloc`, `stbds_strreset` | string longer than selected block; dedicated allocation before/after normal storage exists | [x] |
| 24 | `stbds_stralloc`, `stbds_strreset` | arena block growth reaches 1 MiB maximum | [x] |
| 25 | `stbds_strreset` | empty arena and populated arena | [x] |
| 26 | `stbds_hmput_default` | null binary map; create zero default element | [x] |
| 27 | `stbds_hmput_default` | existing default-only or populated map; preserve map | [x] |
| 28 | `stbds_hmget_key_ts`, `stbds_hmget_key` | null binary map and absent key | [x] |
| 29 | `stbds_hmget_key_ts`, `stbds_hmget_key` | binary map without table and absent key | [x] |
| 30 | `stbds_hmget_key_ts`, `stbds_hmget_key` | binary table: present and absent fixed-width keys | [x] |
| 31 | `stbds_hmput_key` | binary mode `0`, first insertion and distinct new keys | [x] |
| 32 | `stbds_hmput_key` | binary mode `0`, existing-key update (no new element) | [x] |
| 33 | `stbds_hmput_key` | binary mode `0`, enough keys to grow table and array repeatedly | [x] |
| 34 | `stbds_hmput_key` | binary keys with widths `1`, `4`, `8`, and non-word width | [x] |
| 35 | `stbds_hmput_key`, `stbds_hmget_key` | string comparison mode (`mode >= 1`) created implicitly as borrowed/default strings | [x] |
| 36 | `stbds_shmode_func`, map operations | explicit mode `0` (`STBDS_SH_NONE`) with binary key copy | [x] |
| 37 | `stbds_shmode_func`, map operations | explicit mode `1` (`STBDS_SH_DEFAULT`) with borrowed strings | [x] |
| 38 | `stbds_shmode_func`, map operations | explicit mode `2` (`STBDS_SH_STRDUP`) with duplicated strings | [x] |
| 39 | `stbds_shmode_func`, map operations | explicit mode `3` (`STBDS_SH_ARENA`) with arena strings | [x] |
| 40 | `stbds_shmode_func`, map operations | out-of-range mode values (`-1`, `4`, `INT_MAX`) truncated into mode byte; default switch branch | [x] |
| 41 | string map operations | empty, one-byte, long, and high-bit string keys; present/absent lookup | [x] |
| 42 | `stbds_hmdel_key` | null map | [x] |
| 43 | `stbds_hmdel_key` | default-only map with no hash table | [x] |
| 44 | `stbds_hmdel_key` | populated map with absent key | [x] |
| 45 | `stbds_hmdel_key` | delete present final element | [x] |
| 46 | `stbds_hmdel_key` | delete non-final element; move final element and repair its bucket index | [x] |
| 47 | `stbds_hmdel_key`, `stbds_hmput_key` | delete then insert; tombstone reuse/rebuild threshold | [x] |
| 48 | `stbds_hmdel_key` | mass deletion after growth; shrink threshold | [x] |
| 49 | `stbds_hmdel_key` | string modes default/strdup/arena; strdup deletion frees owned key | [x] |
| 50 | `stbds_hmfree_func` | null, default-only, populated binary map | [x] |
| 51 | `stbds_hmfree_func` | populated borrowed/strdup/arena string maps | [x] |
| 52 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` | [x] |
| 53 | `hm_geti` | `num <= 0`, one/few entries, and many entries crossing grow/shrink thresholds | [x] |

