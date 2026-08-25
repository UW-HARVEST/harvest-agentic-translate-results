# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table. `c_src/CMakeLists.txt` has no options,
conditional sources, or compile definitions. There is exactly one valid build
configuration:

| # | Cargo feature set | CMake configuration | [x] |
|---|-------------------|---------------------|-----|
| 1 | empty (`--no-default-features`) | default, PIC enabled | [x] |

## Runtime and Input Configurations

Rows are derived from branches, switches, constants, and public definitions in
`c_src/src/lib.c`. "Randomized" means many fixed-seed generated values within
the listed shape.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array; requested capacity below 4, exercising minimum capacity 4 | [x] |
| 2 | `stbds_arrgrowf` | null array; `addlen > min_cap`, so minimum length wins | [x] |
| 3 | `stbds_arrgrowf` | existing array; requested capacity is already available, returning the same allocation | [x] |
| 4 | `stbds_arrgrowf` | existing array; requested capacity is below twice current capacity, doubling capacity | [x] |
| 5 | `stbds_arrgrowf` | existing array; requested capacity is at least twice current capacity, using the request | [x] |
| 6 | `stbds_arrgrowf` | element widths 1, 4, 8, and 16 bytes with randomized prior contents and lengths | [x] |
| 7 | `stbds_arrfreef` | free each non-null array shape produced by rows 1-6 | [x] |
| 8 | `stbds_rand_seed` | seeds 0, 1, maximum `size_t`, and randomized values before fresh map creation | [x] |
| 9 | `stbds_hash_string` | empty NUL-terminated string | [x] |
| 10 | `stbds_hash_string` | one-byte strings, including bytes `0x01`, `0x7f`, and `0xff` | [x] |
| 11 | `stbds_hash_string` | randomized multi-byte ASCII strings | [x] |
| 12 | `stbds_hash_string` | randomized multi-byte strings containing bytes with the high bit set | [x] |
| 13 | `stbds_hash_bytes` | empty input (`len == 0`) | [x] |
| 14 | `stbds_hash_bytes` | complete 8-byte blocks (`len > 0 && len % 8 == 0`) | [x] |
| 15 | `stbds_hash_bytes` | byte lengths with `len % 8 == 1` | [x] |
| 16 | `stbds_hash_bytes` | byte lengths with `len % 8 == 2` | [x] |
| 17 | `stbds_hash_bytes` | byte lengths with `len % 8 == 3` | [x] |
| 18 | `stbds_hash_bytes` | byte lengths with `len % 8 == 4`, including a high-bit fourth byte | [x] |
| 19 | `stbds_hash_bytes` | byte lengths with `len % 8 == 5` | [x] |
| 20 | `stbds_hash_bytes` | byte lengths with `len % 8 == 6` | [x] |
| 21 | `stbds_hash_bytes` | byte lengths with `len % 8 == 7` | [x] |
| 22 | `stbds_hmfree_func` | non-null raw array with no hash table | [x] |
| 23 | `stbds_hmfree_func` | binary map with a populated hash table | [x] |
| 24 | `stbds_hmfree_func` | string map in `STRDUP` and `ARENA` ownership modes | [x] |
| 25 | `stbds_hmget_key_ts` | null map creates a zero default entry and reports index `-1` | [x] |
| 26 | `stbds_hmget_key_ts` | existing default-only map with no table reports index `-1` | [x] |
| 27 | `stbds_hmget_key_ts` | populated map, randomized present keys in binary and string modes | [x] |
| 28 | `stbds_hmget_key_ts` | populated map, randomized absent keys in binary and string modes | [x] |
| 29 | `stbds_hmget_key` | null map creates a default entry and stores header temp `-1` | [x] |
| 30 | `stbds_hmget_key` | existing default-only map with no table stores header temp `-1` | [x] |
| 31 | `stbds_hmget_key` | populated map, randomized present keys in binary and string modes | [x] |
| 32 | `stbds_hmget_key` | populated map, randomized absent keys in binary and string modes | [x] |
| 33 | `stbds_hmput_default` | null map creates one zeroed default entry | [x] |
| 34 | `stbds_hmput_default` | non-null raw allocation whose header length is zero creates the default entry | [x] |
| 35 | `stbds_hmput_default` | map whose default entry already exists returns unchanged | [x] |
| 36 | `stbds_hmput_key` | first binary insertion into a null map, element widths 8 and 16 | [x] |
| 37 | `stbds_hmput_key` | repeated binary key updates the existing index without increasing length | [x] |
| 38 | `stbds_hmput_key` | randomized binary insertions crossing array-capacity growth | [x] |
| 39 | `stbds_hmput_key` | randomized binary insertions crossing hash-table thresholds 8 to 16 to 32 slots | [x] |
| 40 | `stbds_hmput_key` | colliding binary keys exercise wrapped bucket scans | [x] |
| 41 | `stbds_hmput_key` | insertion after deletion reuses a tombstone | [x] |
| 42 | `stbds_hmput_key` | string mode with default pointer ownership | [x] |
| 43 | `stbds_hmput_key` | string mode with duplicated-string ownership | [x] |
| 44 | `stbds_hmput_key` | string mode with arena ownership | [x] |
| 45 | `stbds_hmput_key` | binary mode values below zero and string mode values above one, matching `mode >= 1` classification | [x] |
| 46 | `stbds_shmode_func` | `STBDS_SH_NONE` (0) | [x] |
| 47 | `stbds_shmode_func` | `STBDS_SH_DEFAULT` (1) | [x] |
| 48 | `stbds_shmode_func` | `STBDS_SH_STRDUP` (2) | [x] |
| 49 | `stbds_shmode_func` | `STBDS_SH_ARENA` (3) | [x] |
| 50 | `stbds_shmode_func` | out-of-range mode values `-1`, 4, and 256, stored after `unsigned char` conversion | [x] |
| 51 | `stbds_hmdel_key` | null map | [x] |
| 52 | `stbds_hmdel_key` | default-only map with no table | [x] |
| 53 | `stbds_hmdel_key` | missing binary and string keys | [x] |
| 54 | `stbds_hmdel_key` | delete final entry, with binary/default-string/strdup-string/arena-string ownership | [x] |
| 55 | `stbds_hmdel_key` | delete non-final entry and repair moved-entry index | [x] |
| 56 | `stbds_hmdel_key` | enough deletions to cross the hash-table shrink threshold | [x] |
| 57 | `stbds_hmdel_key` | enough deletions to cross the tombstone rebuild threshold without shrinking | [x] |
| 58 | `stbds_stralloc` | empty and short strings fit in a newly allocated 512-byte block | [x] |
| 59 | `stbds_stralloc` | repeated randomized strings fit in an existing block | [x] |
| 60 | `stbds_stralloc` | string exactly fills the current block (`len == remaining`) | [x] |
| 61 | `stbds_stralloc` | string exceeds current remaining space but fits the next normal block | [x] |
| 62 | `stbds_stralloc` | string longer than selected block gets a dedicated block, with arena initially empty | [x] |
| 63 | `stbds_stralloc` | dedicated long-string block is linked after normal storage already exists; growth reaches the 1 MiB cap | [x] |
| 64 | `stbds_strreset` | empty zeroed arena | [x] |
| 65 | `stbds_strreset` | arena containing normal and dedicated blocks; all fields become zero | [x] |
| 66 | `strkey` | negative integers | [x] |
| 67 | `strkey` | zero | [x] |
| 68 | `strkey` | positive integers including `INT_MAX` | [x] |
| 69 | `sh_puts` | negative `num`, so allocation loop is empty and map output contains that value | [x] |
| 70 | `sh_puts` | zero `num` | [x] |
| 71 | `sh_puts` | positive `num`, including enough iterations to allocate multiple arena blocks | [x] |
