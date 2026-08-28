# Configuration Surface

There are no Cargo features or C preprocessor feature switches. The rows below
enumerate the runtime branches and input shapes in all 16 exported entry
points. `size_t` is 64-bit in the produced shared objects.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| 1 | `stbds_arrgrowf` | null array; `addlen == 0`, `min_cap == 0`; early return preserves `NULL` | [x] |
| 2 | `stbds_arrgrowf` | null array; `addlen` supplies minimum length and is `1..3` | [x] |
| 3 | `stbds_arrgrowf` | null array; explicit `min_cap >= 4` dominates `addlen` | [x] |
| 4 | `stbds_arrgrowf` | existing array; requested capacity does not exceed capacity; pointer/header unchanged | [x] |
| 5 | `stbds_arrgrowf` | existing array; growth request is below twice capacity; capacity doubles | [x] |
| 6 | `stbds_arrgrowf` | existing array; request is at least twice capacity; requested capacity is used | [x] |
| 7 | `stbds_arrfreef` | free each valid allocation shape produced by rows 1-6 | [x] |
| 8 | `stbds_rand_seed` + map operations | seed `0`, fixed nonzero, and `SIZE_MAX`; newly created table consumes seed | [x] |
| 9 | `stbds_hash_string` | empty C string; varied seeds | [x] |
| 10 | `stbds_hash_string` | nonempty ASCII strings of one and many bytes; varied seeds | [x] |
| 11 | `stbds_hash_string` | non-ASCII bytes (`0x80..0xff`) before NUL; unsigned-byte branch behavior | [x] |
| 12 | `stbds_hash_bytes` | null or nonnull data with `len == 0`; varied seeds | [x] |
| 13 | `stbds_hash_bytes` | lengths `1..7`; every SipHash tail switch case | [x] |
| 14 | `stbds_hash_bytes` | length 8; exactly one full machine-word block and empty tail | [x] |
| 15 | `stbds_hash_bytes` | lengths `9..15`; full block plus every tail shape | [x] |
| 16 | `stbds_hash_bytes` | lengths at least 16; multiple full blocks, including bytes with bit 7 set | [x] |
| 17 | `stbds_hmput_default` | null map; create zero default entry | [x] |
| 18 | `stbds_hmput_default` | existing map with default entry; no-op and pointer preserved | [x] |
| 19 | `stbds_hmget_key_ts` | null map; binary and string modes; create default and return `temp == -1` | [x] |
| 20 | `stbds_hmget_key`, `stbds_hmget_key_ts` | map has default entry but no table; absent key in binary/string modes | [x] |
| 21 | `stbds_hmput_key` + gets | binary mode; null map; key sizes 0, 1, 4, and non-word-aligned many bytes | [x] |
| 22 | `stbds_hmput_key` + gets | binary mode; insert distinct key, update duplicate key, and preserve count | [x] |
| 23 | `stbds_hmput_key` + gets | binary mode; enough distinct keys to grow table at 75% occupancy | [x] |
| 24 | `stbds_hmput_key` + gets | string mode with implicit `STBDS_SH_DEFAULT`; empty/one/many-byte keys | [x] |
| 25 | `stbds_shmode_func` + put/get | explicit `STBDS_SH_DEFAULT` mode; stored key aliases caller string | [x] |
| 26 | `stbds_shmode_func` + put/get | explicit `STBDS_SH_STRDUP` mode; stored key is an independent allocation | [x] |
| 27 | `stbds_shmode_func` + put/get | explicit `STBDS_SH_ARENA` mode; stored key is arena-owned | [x] |
| 28 | `stbds_shmode_func` + put/get | mode `STBDS_SH_NONE` with binary operations | [x] |
| 29 | `stbds_shmode_func` + put/get | out-of-range mode values (`-1`, `4`, `INT_MAX`) and matching operation modes | [x] |
| 30 | `stbds_hmget_key`, `stbds_hmget_key_ts` | populated map; present key and absent key; both binary and string classification | [x] |
| 31 | `stbds_hmdel_key` | null map | [x] |
| 32 | `stbds_hmdel_key` | default-only map with null hash table | [x] |
| 33 | `stbds_hmdel_key` | populated binary/string map; absent key | [x] |
| 34 | `stbds_hmdel_key` | delete final array element; no move needed | [x] |
| 35 | `stbds_hmdel_key` | delete non-final element; move final element and repair its index | [x] |
| 36 | `stbds_hmdel_key` + put | deletion creates tombstone; insertion reuses tombstone | [x] |
| 37 | `stbds_hmdel_key` | enough deletions to cross tombstone rebuild threshold | [x] |
| 38 | `stbds_hmdel_key` | enough deletions after growth to cross used-count shrink threshold | [x] |
| 39 | `stbds_hmdel_key`, `stbds_hmfree_func` | string default, strdup, and arena ownership on deletion/free | [x] |
| 40 | `stbds_hmfree_func` | null, default-only/no-table, empty table, and nonempty binary/string tables | [x] |
| 41 | `stbds_stralloc` | zeroed arena; empty and short strings allocate initial 512-byte block | [x] |
| 42 | `stbds_stralloc` | string fits current `remaining`; allocations share current block | [x] |
| 43 | `stbds_stralloc` | string exhausts current block; block size growth below 1 MiB maximum | [x] |
| 44 | `stbds_stralloc` | `len > blocksize`; dedicated oversized block with empty and nonempty arena | [x] |
| 45 | `stbds_stralloc` | block growth reaches 1 MiB maximum and stops incrementing block counter | [x] |
| 46 | `stbds_strreset` | zeroed arena and arena with one/many standard/dedicated blocks | [x] |
| 47 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` numbers | [x] |
| 48 | `strkey` | repeated calls overwrite and return the same static 256-byte buffer | [x] |
| 49 | `sh_puts` | `num <= 0`; allocation loop skipped, output still contains one map entry | [x] |
| 50 | `sh_puts` | `num == 1`, moderate many, and enough iterations to grow arena blocks | [x] |
