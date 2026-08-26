# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and CMake has no options or conditional
definitions. There is exactly one valid build configuration:

| # | default features | explicit features | CMake options | [ ] |
|---|------------------|-------------------|---------------|-----|
| 1 | disabled | none (empty set) | none | [x] |

## Runtime Configurations

Rows are derived from public exports and the `if`/`switch` branches in
`c_src/src/lib.c`. Element sizes and counts represent shape axes; randomized
values and seeds are used within each row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array; `addlen = min_cap = 0`, so no allocation | [x] |
| 2 | `stbds_arrgrowf`, `stbds_arrfreef` | null array; effective capacity 1..3; element sizes 1, 4, and 16; minimum capacity rounds to 4 | [x] |
| 3 | `stbds_arrgrowf`, `stbds_arrfreef` | null array; effective capacity at least 4; requested capacity and add length dominance | [x] |
| 4 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array; effective capacity does not exceed current capacity; pointer/header/content unchanged | [x] |
| 5 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array; needed capacity below twice current capacity; capacity doubles and bytes survive | [x] |
| 6 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array; needed capacity at least twice current capacity; exact requested capacity and bytes survive | [x] |
| 7 | `stbds_hash_bytes` | null data with zero length; random seeds | [x] |
| 8 | `stbds_hash_bytes` | tail-only lengths 1..7; random byte values and seeds | [x] |
| 9 | `stbds_hash_bytes` | exact 8-byte block and block-plus-tail lengths 9..15 | [x] |
| 10 | `stbds_hash_bytes` | multiple full blocks and tails; lengths 16, 17, 31, 32, 63, 64, and 257 | [x] |
| 11 | `stbds_hash_string` | empty NUL-terminated string; random seeds | [x] |
| 12 | `stbds_hash_string` | short/long NUL-terminated strings, including bytes with the high bit set | [x] |
| 13 | `stbds_rand_seed`, `stbds_hmput_key` | seed 0, fixed nonzero, and random values before creating a fresh table | [x] |
| 14 | `stbds_hmget_key_ts`, `stbds_hmget_key`, `stbds_hmfree_func` | null binary map lookup; default entry creation and missing sentinel | [x] |
| 15 | `stbds_hmput_default`, `stbds_hmget_key_ts`, `stbds_hmfree_func` | null map then existing map with no hash table; default entry is zeroed and not duplicated | [x] |
| 16 | `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`, `stbds_hmfree_func` | binary mode 0; key sizes 1, 4, 8, and 16; one insert then hit/miss lookup | [x] |
| 17 | same as row 16 | binary mode 0; duplicate key update path and many distinct values | [x] |
| 18 | same as row 16 | binary mode 0; enough inserts to cross 8-slot, 16-slot, and larger growth thresholds | [x] |
| 19 | `stbds_shmode_func`, `stbds_hmput_key`, lookup functions, `stbds_hmfree_func` | string mode `STBDS_SH_DEFAULT`; empty, short, and long keys; borrowed key pointers | [x] |
| 20 | same as row 19 | string mode `STBDS_SH_STRDUP`; copied keys, duplicate insert, and lookup | [x] |
| 21 | same as row 19 | string mode `STBDS_SH_ARENA`; empty/small/oversized keys and lookup | [x] |
| 22 | same as row 19 | integer modes below 0 and above named range; C branch rule `mode >= STBDS_HM_STRING` | [x] |
| 23 | `stbds_hmdel_key` | null map and map with default element but no hash table | [x] |
| 24 | `stbds_hmdel_key` | binary table; absent key and present first/middle/last keys | [x] |
| 25 | `stbds_hmdel_key` | binary table; deletion moves the final entry and repairs its index | [x] |
| 26 | `stbds_hmdel_key`, `stbds_hmput_key` | binary table; delete then insert reuses a tombstone | [x] |
| 27 | `stbds_hmdel_key` | enough binary entries/deletions to trigger table shrink | [x] |
| 28 | `stbds_hmdel_key` | deletion pattern triggers same-size tombstone rebuild | [x] |
| 29 | `stbds_hmdel_key` | string modes default, strdup, and arena; absent/present keys and moved entries | [x] |
| 30 | `stbds_hmdel_key` | key offsets 0 and nonzero with binary element shapes | [x] |
| 31 | `stbds_hmfree_func` | null raw array; binary map; default/strdup/arena string maps | [x] |
| 32 | `stbds_stralloc` | zeroed arena; string storage lengths 1, 2, 511, and 512 bytes including NUL | [x] |
| 33 | `stbds_stralloc` | reuse current block with exact-fit and non-exact-fit remaining space | [x] |
| 34 | `stbds_stralloc` | allocation length above current block size, both with empty and populated arena | [x] |
| 35 | `stbds_stralloc` | repeated allocations grow block exponent through the 1 MiB cap | [x] |
| 36 | `stbds_strreset` | zeroed arena and arena containing normal plus oversized blocks | [x] |
| 37 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` values; static buffer replacement | [x] |
| 38 | `arr_push` | negative, zero, 1..49, exactly 50, 51+, and multi-iteration counts | [x] |
