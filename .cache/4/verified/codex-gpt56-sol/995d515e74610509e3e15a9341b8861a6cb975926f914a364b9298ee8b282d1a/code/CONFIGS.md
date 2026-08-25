# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and CMake declares no options. The full
valid feature set is therefore:

| # | Cargo invocation feature set | C configuration |
|---|------------------------------|-----------------|
| 1 | `--no-default-features` (empty set) | default, position-independent shared library |

## Runtime Configurations

The rows below are derived from every exported C entry point and the branches
on null/existing state, mode, size, count, probe position, capacity, and arena
block boundaries. `HM_BINARY = 0`, `HM_STRING = 1`, `SH_NONE = 0`,
`SH_DEFAULT = 1`, `SH_STRDUP = 2`, and `SH_ARENA = 3`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array; `addlen == 0`; requested capacity 0 returns `NULL`, while 1..3 exercise minimum capacity 4 | [x] |
| 2 | `stbds_arrgrowf` | null array; nonzero add length larger than requested capacity | [x] |
| 3 | `stbds_arrgrowf` | existing array; requested minimum does not exceed capacity; pointer and header unchanged | [x] |
| 4 | `stbds_arrgrowf` | existing array; required capacity is below twice current capacity; doubles | [x] |
| 5 | `stbds_arrgrowf` | existing array; requested capacity is at least twice current capacity; uses request | [x] |
| 6 | `stbds_arrgrowf`, `stbds_arrfreef` | element widths 1, 2, 4, 8, 16 and non-power-of-two; retained payload bytes across reallocation | [x] |
| 7 | `stbds_rand_seed` | seeds `0`, `1`, maximum `size_t`, and randomized values before first table creation | [x] |
| 8 | `stbds_hash_string` | empty NUL-terminated string across randomized seeds | [x] |
| 9 | `stbds_hash_string` | one-byte and many-byte strings, including bytes `0x01..0xff` before NUL | [x] |
| 10 | `stbds_hash_bytes` | zero bytes with non-null and null pointers across randomized seeds | [x] |
| 11 | `stbds_hash_bytes` | tail lengths 1 through 7, covering every switch arm | [x] |
| 12 | `stbds_hash_bytes` | exactly one `size_t` word | [x] |
| 13 | `stbds_hash_bytes` | multiple full words plus tail lengths 0 through 7 | [x] |
| 14 | `stbds_hmget_key_ts` | null map, binary mode; creates zero default element and reports temp `-1` | [x] |
| 15 | `stbds_hmget_key` | null map, binary mode; stores temp `-1` in header | [x] |
| 16 | `stbds_hmput_default` | null map; creates one zeroed default element | [x] |
| 17 | `stbds_hmput_default` | existing map; default already exists; no state change | [x] |
| 18 | `stbds_hmget_key`, `stbds_hmget_key_ts` | map has default element but no hash table | [x] |
| 19 | `stbds_hmput_key` | null map, binary mode, key widths 0, 1, 2, 4, 8, 16 | [x] |
| 20 | `stbds_hmput_key` | binary mode inserts new unique key into empty slot | [x] |
| 21 | `stbds_hmput_key` | binary mode updates existing key without increasing length | [x] |
| 22 | `stbds_hmget_key`, `stbds_hmget_key_ts` | binary key found in first probe segment | [x] |
| 23 | `stbds_hmget_key`, `stbds_hmget_key_ts` | binary key found after wrapped probe segment | [x] |
| 24 | `stbds_hmget_key`, `stbds_hmget_key_ts` | binary key absent, empty slot in first probe segment | [x] |
| 25 | `stbds_hmget_key`, `stbds_hmget_key_ts` | binary key absent, empty slot after wrapped probe segment | [x] |
| 26 | `stbds_hmput_key` | enough unique inserts to cross 8-slot load threshold and grow repeatedly | [x] |
| 27 | `stbds_hmput_key` | insert after deletion reuses a tombstone | [x] |
| 28 | `stbds_hmdel_key` | null map | [x] |
| 29 | `stbds_hmdel_key` | map has default entry but no hash table | [x] |
| 30 | `stbds_hmdel_key` | initialized binary table, missing key | [x] |
| 31 | `stbds_hmdel_key` | delete existing last element; no move | [x] |
| 32 | `stbds_hmdel_key` | delete existing non-last element; moves final element and repairs index | [x] |
| 33 | `stbds_hmdel_key` | deletions cross shrink threshold while slot count is greater than 8 | [x] |
| 34 | `stbds_hmdel_key` | deletions cross tombstone rebuild threshold without shrinking | [x] |
| 35 | `stbds_hmfree_func` | null map | [x] |
| 36 | `stbds_hmfree_func` | map with default only and no table | [x] |
| 37 | `stbds_hmfree_func` | populated binary map | [x] |
| 38 | `stbds_shmode_func` | `SH_NONE`, `SH_DEFAULT`, `SH_STRDUP`, and `SH_ARENA` initial states | [x] |
| 39 | `stbds_shmode_func` | out-of-range signed modes and values that truncate to `0..3` | [x] |
| 40 | `stbds_hmput_key` | `HM_STRING` with `SH_DEFAULT`; empty, one-byte, and long keys | [x] |
| 41 | `stbds_hmput_key` | `HM_STRING` with `SH_STRDUP`; key bytes copied and source pointer not retained | [x] |
| 42 | `stbds_hmput_key` | `HM_STRING` with `SH_ARENA`; key bytes arena-allocated | [x] |
| 43 | `stbds_hmput_key` | string lookup/update of existing key with a different source pointer | [x] |
| 44 | `stbds_hmget_key`, `stbds_hmget_key_ts` | string key present and absent for each string storage mode | [x] |
| 45 | `stbds_hmdel_key` | delete string key in default mode | [x] |
| 46 | `stbds_hmdel_key` | delete string key in strdup mode, freeing owned key | [x] |
| 47 | `stbds_hmdel_key` | delete string key in arena mode | [x] |
| 48 | `stbds_hmfree_func` | populated string map in default, strdup, and arena modes | [x] |
| 49 | hash-map entry points | modes below `HM_STRING` use binary hashing/comparison | [x] |
| 50 | hash-map entry points | modes above `HM_STRING` use string hashing/comparison, except delete's strdup free requires exact mode 1 | [x] |
| 51 | hash-map entry points | entry sizes 8, 16, 24 and key offsets 0 and nonzero on deletion | [x] |
| 52 | `stbds_stralloc` | zeroed arena; empty and short strings allocate first 512-byte block | [x] |
| 53 | `stbds_stralloc` | existing block has enough remaining bytes | [x] |
| 54 | `stbds_stralloc` | existing block lacks space; allocates next geometrically sized normal block | [x] |
| 55 | `stbds_stralloc` | string length exceeds current block size with empty arena; dedicated block becomes storage | [x] |
| 56 | `stbds_stralloc` | string length exceeds current block size with existing storage; dedicated block links after head | [x] |
| 57 | `stbds_stralloc` | repeated allocations advance block counter until 1 MiB maximum and stop incrementing | [x] |
| 58 | `stbds_strreset` | empty arena | [x] |
| 59 | `stbds_strreset` | arena with one normal block, multiple blocks, and dedicated oversized blocks | [x] |
| 60 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` values | [x] |
| 61 | `strkey` | repeated calls overwrite and return the same static 256-byte buffer | [x] |
| 62 | `str_dups` | negative and zero count; arena loop is empty | [x] |
| 63 | `str_dups` | positive count below, at, and above arena block boundaries | [x] |
| 64 | `str_dups` | randomized counts; printed key/value output is byte-identical | [x] |
