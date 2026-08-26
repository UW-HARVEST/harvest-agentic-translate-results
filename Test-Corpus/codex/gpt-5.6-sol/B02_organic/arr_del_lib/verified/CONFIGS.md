# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
configuration options or conditional sources. There is exactly one valid
feature combination:

| # | Cargo feature combination | C configuration | verified |
|---|---------------------------|-----------------|----------|
| 1 | empty (`--no-default-features`) | default CMake build | [x] |

## Runtime and Input Configurations

Rows are the branch-distinct combinations exposed by the C dynamic symbols.
`size_t` is 64-bit in the tested ABI.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | Null array; zero add length; minimum capacity 0 returns null, while 1-3 allocate capacity 4 | [x] |
| 2 | `stbds_arrgrowf` | Null array; add length exceeds requested capacity | [x] |
| 3 | `stbds_arrgrowf` | Existing array; requested capacity does not exceed capacity (same pointer/header) | [x] |
| 4 | `stbds_arrgrowf` | Existing array; growth request below twice current capacity (doubling path) | [x] |
| 5 | `stbds_arrgrowf` | Existing array; growth request at or above twice current capacity | [x] |
| 6 | `stbds_arrfreef` | Free a valid allocation from `stbds_arrgrowf` | [x] |
| 7 | `stbds_rand_seed` | Seeds 0, 1, high-bit set, and `SIZE_MAX`; observed through newly created hash tables | [x] |
| 8 | `stbds_hash_string` | Empty C string across varied seeds | [x] |
| 9 | `stbds_hash_string` | Nonempty ASCII strings: one byte and many bytes | [x] |
| 10 | `stbds_hash_string` | Nonempty strings containing bytes with the high bit set | [x] |
| 11 | `stbds_hash_bytes` | Length 0 with null and nonnull pointers | [x] |
| 12 | `stbds_hash_bytes` | Tail lengths 1 through 7 | [x] |
| 13 | `stbds_hash_bytes` | Exactly one full `size_t` block (8 bytes) | [x] |
| 14 | `stbds_hash_bytes` | One full block plus tail lengths 1 through 7 | [x] |
| 15 | `stbds_hash_bytes` | Multiple full blocks, with and without a tail | [x] |
| 16 | `stbds_hmput_default` | Null map creates one zeroed default element | [x] |
| 17 | `stbds_hmput_default` | Existing map with default entry returns unchanged | [x] |
| 18 | `stbds_hmget_key_ts` | Null map creates default entry and reports `-1` | [x] |
| 19 | `stbds_hmget_key_ts`, `stbds_hmget_key` | Existing default-only map with no hash table reports missing | [x] |
| 20 | `stbds_hmput_key`, `stbds_hmget_key_ts`, `stbds_hmget_key` | Binary mode; insert then hit for element/key widths 1, 4, 8, and 16 bytes | [x] |
| 21 | `stbds_hmput_key`, `stbds_hmget_key_ts`, `stbds_hmget_key` | Binary mode; populated map lookup miss | [x] |
| 22 | `stbds_hmput_key` | Binary mode; update an existing key without increasing length | [x] |
| 23 | `stbds_hmput_key` | Binary mode; enough unique inserts to grow array and hash table | [x] |
| 24 | `stbds_hmput_key` | Binary mode; insert after deletion reuses a tombstone | [x] |
| 25 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key` | String mode with `STBDS_SH_DEFAULT` borrowed keys; empty, short, and long strings | [x] |
| 26 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key` | String mode with `STBDS_SH_STRDUP`; empty, short, and long strings | [x] |
| 27 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key` | String mode with `STBDS_SH_ARENA`; empty, short, and long strings | [x] |
| 28 | `stbds_shmode_func`, `stbds_hmput_key` | Mode `STBDS_SH_NONE` and out-of-range mode values use the switch default copy path | [x] |
| 29 | `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmdel_key` | Hash mode below 1 is binary; modes 1 and one-past-range values are string | [x] |
| 30 | `stbds_hmdel_key` | Null map | [x] |
| 31 | `stbds_hmdel_key` | Default-only map with no table | [x] |
| 32 | `stbds_hmdel_key` | Populated map; missing key | [x] |
| 33 | `stbds_hmdel_key` | Delete final element (no element move) | [x] |
| 34 | `stbds_hmdel_key` | Delete non-final binary element (move final element and repair index) | [x] |
| 35 | `stbds_hmdel_key` | Delete non-final string element in default, strdup, and arena ownership modes | [x] |
| 36 | `stbds_hmdel_key` | Delete enough entries to shrink a grown table | [x] |
| 37 | `stbds_hmdel_key` | Accumulate enough tombstones to rebuild at the same table size | [x] |
| 38 | `stbds_hmfree_func` | Null pointer and default-only binary map | [x] |
| 39 | `stbds_hmfree_func` | Populated binary map | [x] |
| 40 | `stbds_hmfree_func` | Populated string map in default, strdup, and arena modes | [x] |
| 41 | `stbds_stralloc` | Empty arena; normal allocation up to 512 bytes | [x] |
| 42 | `stbds_stralloc` | Existing arena block has enough remaining space | [x] |
| 43 | `stbds_stralloc` | Existing block lacks space; allocate the next geometrically sized block | [x] |
| 44 | `stbds_stralloc` | String larger than current block size; dedicated oversized block with empty/nonempty arena | [x] |
| 45 | `stbds_stralloc` | Arena block counter reaches the 1 MiB growth cap | [x] |
| 46 | `stbds_strreset` | Empty arena, one block, and linked multiple blocks | [x] |
| 47 | `strkey` | Negative, zero, positive, `INT_MIN`, and `INT_MAX` | [x] |
| 48 | `arr_del` | Negative, zero, positive, `INT_MIN`, and `INT_MAX`; all four deletion indices | [x] |
