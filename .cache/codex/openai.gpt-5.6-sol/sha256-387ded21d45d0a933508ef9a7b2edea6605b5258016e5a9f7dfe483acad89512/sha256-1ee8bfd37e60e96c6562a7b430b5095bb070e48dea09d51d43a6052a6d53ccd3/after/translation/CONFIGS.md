# Configuration Surface

Mechanically derived from the branches, switches, constants, and exported
entry points in `src/lib.c`. The crate declares no Cargo features, so the only
feature configuration is the empty/default feature set; it is exercised both
normally and with `--no-default-features`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|---|
| V01 | `stbds_arrgrowf`, `stbds_arrfreef` | null array; requested capacity below 4, exercising minimum capacity 4 | [x] |
| V02 | `stbds_arrgrowf`, `stbds_arrfreef` | null array; `addlen > min_cap`, so required length wins | [x] |
| V03 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array; request does not exceed capacity, pointer/capacity unchanged | [x] |
| V04 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array; request below twice capacity, so capacity doubles and bytes persist | [x] |
| V05 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array; request at/above twice capacity, explicit minimum wins | [x] |
| V06 | `stbds_hash_string` | empty C string; randomized seeds | [x] |
| V07 | `stbds_hash_string` | nonempty ASCII strings; randomized lengths/seeds | [x] |
| V08 | `stbds_hash_string` | nonempty strings containing bytes `0x80..0xff` before NUL | [x] |
| V09 | `stbds_hash_bytes` | length 0 (tail switch case 0), including null pointer | [x] |
| V10 | `stbds_hash_bytes` | length 1 (tail case 1) | [x] |
| V11 | `stbds_hash_bytes` | length 2 (tail case 2) | [x] |
| V12 | `stbds_hash_bytes` | length 3 (tail case 3) | [x] |
| V13 | `stbds_hash_bytes` | length 4 (tail case 4, signed promoted high byte) | [x] |
| V14 | `stbds_hash_bytes` | length 5 (tail case 5) | [x] |
| V15 | `stbds_hash_bytes` | length 6 (tail case 6) | [x] |
| V16 | `stbds_hash_bytes` | length 7 (tail case 7) | [x] |
| V17 | `stbds_hash_bytes` | exactly one full `size_t` block (length 8) | [x] |
| V18 | `stbds_hash_bytes` | full block plus tails 1 through 7 (lengths 9..15) | [x] |
| V19 | `stbds_hash_bytes` | multiple full blocks, with and without a tail | [x] |
| V20 | `stbds_rand_seed`, `stbds_hmput_key` | seed 0 and nonzero seeds applied to newly created binary tables | [x] |
| V21 | `stbds_hmget_key_ts`, `stbds_hmfree_func` | null binary map lookup creates default element and reports index `-1` | [x] |
| V22 | `stbds_hmget_key` | existing default-only map with no table; binary lookup reports `header.temp = -1` | [x] |
| V23 | `stbds_hmput_default` | null map creates one zeroed default element; second call is idempotent | [x] |
| V24 | `stbds_hmput_key`, `stbds_hmget_key_ts`, `stbds_hmget_key` | binary keys; key widths 1, 2, 4, 8, and 16 bytes; insert then found/missing lookup | [x] |
| V25 | `stbds_hmput_key` | binary duplicate key updates existing slot rather than increasing length | [x] |
| V26 | `stbds_hmput_key` | binary inserts cross 6/8 load threshold, growing and rehashing table | [x] |
| V27 | `stbds_hmdel_key` | binary delete on null map, no-table map, and missing key | [x] |
| V28 | `stbds_hmdel_key` | binary delete final array element versus non-final element moved into its slot | [x] |
| V29 | `stbds_hmdel_key`, `stbds_hmput_key` | binary deletion creates tombstone and later insertion reuses it | [x] |
| V30 | `stbds_hmdel_key` | enough tombstones at slot count 8 to trigger same-size table rebuild | [x] |
| V31 | `stbds_hmdel_key` | grown table falls below shrink threshold and halves | [x] |
| V32 | `stbds_shmode_func`, `stbds_hmfree_func` | modes 0 (`NONE`), 1 (`DEFAULT`), 2 (`STRDUP`), and 3 (`ARENA`) create/free empty tables | [x] |
| V33 | `stbds_shmode_func` | out-of-range mode integers are stored after conversion to `unsigned char` | [x] |
| V34 | `stbds_hmput_key`, `stbds_hmget_key_ts` | implicit string mode (`mode >= 1`) stores borrowed key (`SH_DEFAULT`) | [x] |
| V35 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key` | explicit `SH_STRDUP`; caller mutates original key after insertion | [x] |
| V36 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key` | explicit `SH_ARENA`; empty, short, and randomized string keys | [x] |
| V37 | `stbds_hmput_key` | string duplicate key under default, strdup, and arena ownership modes | [x] |
| V38 | `stbds_hmdel_key` | string found/missing deletes; exact mode 1 versus out-of-range string mode 2 | [x] |
| V39 | `stbds_hmfree_func` | null pointer, raw dynamic array with no table, and binary/string tables | [x] |
| V40 | `stbds_stralloc` | zeroed arena; empty and short strings fit the current 512-byte block | [x] |
| V41 | `stbds_stralloc` | repeated small strings consume a block and allocate subsequent growing blocks | [x] |
| V42 | `stbds_stralloc` | string longer than current block, with arena storage absent versus present | [x] |
| V43 | `stbds_stralloc`, `stbds_strreset` | lengths around 512 and 1 MiB block constants; reset restores all-zero arena | [x] |
| V44 | `strkey` | negative, zero, and positive `int` values overwrite and return the static buffer | [x] |
| V45 | `sh_geti` | negative and zero counts (empty workflow) | [x] |
| V46 | `sh_geti` | positive counts: empty/one/many parity shapes, table growth, selective deletes, strdup and arena passes | [x] |

## Differential Test Mapping

- V01-V19: `valid_array_and_hash_surface_v01_v19`
- V20-V31: `valid_binary_map_surface_v20_v31_and_e01_e08`
- V32-V39: `valid_string_map_surface_v32_v39`
- V40-V46: `valid_arena_and_public_workflow_surface_v40_v46`

Every symbol call in these tests is resolved from both shared objects through
`libloading`; no translated Rust function is linked or called directly.
