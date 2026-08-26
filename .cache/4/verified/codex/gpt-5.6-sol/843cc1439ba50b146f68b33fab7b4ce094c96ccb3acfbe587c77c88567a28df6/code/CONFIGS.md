# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no options
or conditional definitions. There is exactly one valid feature combination:

| # | Cargo feature set | C configuration | status |
|---|-------------------|-----------------|-----|
| 1 | empty (`--no-default-features`) | default CMake build | [x] |

## Runtime and Input Configurations

Rows are derived from public exports plus the `if`/`switch` branches and
constants in `c_src/src/lib.c`. `size_t` is 64-bit in the required build.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | null data, length 0; empty tail case | [x] |
| 2 | `stbds_hash_bytes` | non-null data, tail lengths 1 through 7, randomized bytes/seeds | [x] |
| 3 | `stbds_hash_bytes` | lengths 8, 16, and many blocks; exact block loop with tail 0 | [x] |
| 4 | `stbds_hash_bytes` | block(s) plus tail lengths 1 through 7 | [x] |
| 5 | `stbds_hash_string` | empty NUL-terminated string, seeds 0 and `SIZE_MAX` | [x] |
| 6 | `stbds_hash_string` | randomized ASCII and high-bit bytes, varied lengths/seeds | [x] |
| 7 | `stbds_rand_seed`, `stbds_hmput_key` | seeds 0, default `0x31415926`, and `SIZE_MAX` before first table allocation | [x] |
| 8 | `stbds_arrgrowf` | null array with `addlen == 0`, `min_cap == 0`; no allocation | [x] |
| 9 | `stbds_arrgrowf`, `stbds_arrfreef` | null array, requested capacity 1 through 3; minimum capacity becomes 4 | [x] |
| 10 | `stbds_arrgrowf`, `stbds_arrfreef` | null array, requested capacity at least 4; exact requested capacity | [x] |
| 11 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array, request within capacity; return same allocation unchanged | [x] |
| 12 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array, request above capacity but below double; capacity doubles and bytes persist | [x] |
| 13 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array, request at least double; exact requested capacity and bytes persist | [x] |
| 14 | `stbds_hmget_key_ts`, `stbds_hmfree_func` | null map, binary mode 0; create zero default entry and report `temp = -1` | [x] |
| 15 | `stbds_hmget_key`, `stbds_hmput_default`, `stbds_hmfree_func` | null then already-initialized binary map; default entry initialization/idempotence | [x] |
| 16 | `stbds_hmget_key`, `stbds_hmget_key_ts` | binary key present versus absent, key sizes 1, 4, 8, and non-word-sized | [x] |
| 17 | `stbds_hmput_key` | binary mode `< 1` including 0 and out-of-range negative values; insert and duplicate update | [x] |
| 18 | `stbds_hmput_key` | string comparison mode `>= 1` including 1 and out-of-range positive values; default borrowed-key storage | [x] |
| 19 | `stbds_hmput_key` | 1 through 5 entries; initial 8-slot table below growth threshold | [x] |
| 20 | `stbds_hmput_key` | 7th and later distinct entries; table growth/rehash, randomized many-entry maps | [x] |
| 21 | `stbds_hmput_key`, `stbds_hmdel_key` | delete a present binary key at final array index | [x] |
| 22 | `stbds_hmput_key`, `stbds_hmdel_key` | delete a present binary key not at final index; move final entry and repair bucket index | [x] |
| 23 | `stbds_hmput_key`, `stbds_hmdel_key` | delete absent key and delete from null/table-less map | [x] |
| 24 | `stbds_hmput_key`, `stbds_hmdel_key` | delete then insert into a tombstone | [x] |
| 25 | `stbds_hmput_key`, `stbds_hmdel_key` | enough deletions to exceed tombstone threshold; same-size table rebuild | [x] |
| 26 | `stbds_hmput_key`, `stbds_hmdel_key` | grow above 8 slots then delete below quarter occupancy; table shrink | [x] |
| 27 | `stbds_hmdel_key` | binary key at nonzero `keyoffset`, matching duplicated key bytes in entry | [x] |
| 28 | `stbds_shmode_func`, `stbds_hmput_key` | `STBDS_SH_DEFAULT` (1), borrowed string keys, empty/short/long strings | [x] |
| 29 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmfree_func` | `STBDS_SH_STRDUP` (2), copied string keys survive source mutation | [x] |
| 30 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmfree_func` | `STBDS_SH_ARENA` (3), arena-backed keys across block allocations | [x] |
| 31 | `stbds_shmode_func`, `stbds_hmput_key` | modes 0, -1, 4, 255, and 256; unsigned-char truncation plus switch default | [x] |
| 32 | `stbds_hmdel_key` | string mode deletion in default, strdup, and arena storage; final/non-final keys | [x] |
| 33 | `stbds_hmdel_key` | out-of-range delete modes `< 1` and `> 1`, exercising exact `mode == 1` branch | [x] |
| 34 | `stbds_hmfree_func` | null, default-only table, binary table, and all three string storage modes | [x] |
| 35 | `stbds_stralloc`, `stbds_strreset` | zeroed arena; empty and short strings fitting 512-byte initial block | [x] |
| 36 | `stbds_stralloc`, `stbds_strreset` | repeated strings exhaust blocks; block exponent increments up to 1 MiB cap | [x] |
| 37 | `stbds_stralloc`, `stbds_strreset` | string longer than current block with empty arena | [x] |
| 38 | `stbds_stralloc`, `stbds_strreset` | string longer than current block with existing arena storage; side-block insertion | [x] |
| 39 | `stbds_strreset` | zeroed empty arena and populated arena; all fields reset to zero | [x] |
| 40 | `strkey` | `INT_MIN`, negative, zero, positive, and `INT_MAX`; exact decimal bytes | [x] |
| 41 | `arr_ins` | randomized `int` values including `INT_MIN`, zero, and `INT_MAX`; all five insertion positions | [x] |

All rows are exercised by `tests/differential.rs`. Every function pointer,
including Rust functions, is resolved from its shared library with
`libloading`; the tests never link to or call the Rust crate directly.
Randomized cases use fixed xorshift seeds and deterministic operation counts.
