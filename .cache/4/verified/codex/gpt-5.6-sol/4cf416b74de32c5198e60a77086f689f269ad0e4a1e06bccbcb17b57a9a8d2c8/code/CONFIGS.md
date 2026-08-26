# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no options,
conditional definitions, or conditional sources. There is exactly one valid
build-time combination:

| # | Cargo features | CMake configuration |
|---|----------------|---------------------|
| 1 | Empty set (`--no-default-features`) | Default shared-library build |

## Runtime and Input Configurations

Rows are derived from branches in `c_src/src/lib.c`. Mode values are C `int`
values, so the invalid-enum partitions (`< 0`, `4..INT_MAX`) are included.
Hash byte-length rows are the cross-product of the full-word loop being skipped
or taken and all eight switch remainders.

| # | entry point(s) | configuration (options set + input shape) | covered |
|---|----------------|--------------------------------------------|---------|
| 1 | `stbds_rand_seed` | Seeds `0`, ordinary nonzero, and `SIZE_MAX`, observed through subsequently created maps | [x] |
| 2 | `stbds_hash_string` | Empty NUL-terminated string; varied seeds | [x] |
| 3 | `stbds_hash_string` | Nonempty strings containing low and high-bit bytes; varied lengths/seeds | [x] |
| 4 | `stbds_hash_bytes` | No full word, remainder 0 (`len == 0`, including null data) | [x] |
| 5 | `stbds_hash_bytes` | No full word, remainder 1 | [x] |
| 6 | `stbds_hash_bytes` | No full word, remainder 2 | [x] |
| 7 | `stbds_hash_bytes` | No full word, remainder 3 | [x] |
| 8 | `stbds_hash_bytes` | No full word, remainder 4 | [x] |
| 9 | `stbds_hash_bytes` | No full word, remainder 5 | [x] |
| 10 | `stbds_hash_bytes` | No full word, remainder 6 | [x] |
| 11 | `stbds_hash_bytes` | No full word, remainder 7 | [x] |
| 12 | `stbds_hash_bytes` | One or more full words, remainder 0 | [x] |
| 13 | `stbds_hash_bytes` | One or more full words, remainder 1 | [x] |
| 14 | `stbds_hash_bytes` | One or more full words, remainder 2 | [x] |
| 15 | `stbds_hash_bytes` | One or more full words, remainder 3 | [x] |
| 16 | `stbds_hash_bytes` | One or more full words, remainder 4 | [x] |
| 17 | `stbds_hash_bytes` | One or more full words, remainder 5 | [x] |
| 18 | `stbds_hash_bytes` | One or more full words, remainder 6 | [x] |
| 19 | `stbds_hash_bytes` | One or more full words, remainder 7 | [x] |
| 20 | `stbds_arrgrowf` | Null array; zero `addlen` and `min_cap`, taking the early return and returning `NULL` | [x] |
| 21 | `stbds_arrgrowf` | Null array; `addlen > min_cap`, including required lengths below 4 (minimum capacity 4) and above 4 | [x] |
| 22 | `stbds_arrgrowf` | Null array; explicit `min_cap >= 4` controls capacity; varied element widths | [x] |
| 23 | `stbds_arrgrowf` | Existing array; requested capacity already available, returning the same allocation | [x] |
| 24 | `stbds_arrgrowf` | Existing array; requested capacity below twice the old capacity, selecting doubling | [x] |
| 25 | `stbds_arrgrowf` | Existing array; requested capacity at least twice the old capacity, selecting request | [x] |
| 26 | `stbds_arrfreef` | Free every nonnull array shape produced by rows 20-25 | [x] |
| 27 | `stbds_hmget_key_ts`, `stbds_hmget_key` | Null map with binary mode and null/non-null key shapes; creates zeroed default entry and reports `-1` | [x] |
| 28 | `stbds_hmput_default` | Null map, existing zero-length backing array, and already initialized map; first two create a default, last is a no-op | [x] |
| 29 | `stbds_hmget_key_ts`, `stbds_hmget_key` | Initialized map with no hash table; missing lookup via `_ts` and header `temp` | [x] |
| 30 | `stbds_hmput_key`, get APIs | Binary mode `0`; new key, existing-key update, found key, and missing key; key widths 0/1/4/8/16 | [x] |
| 31 | `stbds_hmput_key`, get APIs | Out-of-range negative modes (`INT_MIN..-1`), which C treats as binary | [x] |
| 32 | `stbds_hmput_key`, get APIs | Binary insertion counts crossing 6/12/24 load thresholds; collision and wrapped-probe paths | [x] |
| 33 | `stbds_hmdel_key` | Null map, map without table, and table with missing key | [x] |
| 34 | `stbds_hmdel_key` | Found binary key at final index versus non-final index; `keyoffset == 0` | [x] |
| 35 | `stbds_hmdel_key` | Nonzero `keyoffset` with matching duplicated key bytes in the entry | [x] |
| 36 | `stbds_hmdel_key`, `stbds_hmput_key` | Delete then insert through a tombstone; tombstone-rebuild threshold | [x] |
| 37 | `stbds_hmdel_key` | Delete enough entries from a grown table to cross the shrink threshold | [x] |
| 38 | `stbds_shmode_func`, map APIs | String mode `STBDS_SH_DEFAULT` (1): borrowed key pointers; insert/update/get/delete | [x] |
| 39 | `stbds_shmode_func`, map APIs | String mode `STBDS_SH_STRDUP` (2): copied keys; insert/update/get/delete/free | [x] |
| 40 | `stbds_shmode_func`, map APIs | String mode `STBDS_SH_ARENA` (3): arena keys; insert/update/get/delete/free | [x] |
| 41 | `stbds_shmode_func`, map APIs | Out-of-range modes `0`, `4`, `255`, and `INT_MAX`; exact switch-default and `mode >= 1` behavior | [x] |
| 42 | string map APIs | Insertion counts crossing growth thresholds, missing/found probes, non-final deletion, and duplicate textual keys | [x] |
| 43 | `stbds_hmfree_func` | Null map, default-only map, binary map, and each string ownership mode | [x] |
| 44 | `stbds_stralloc` | Empty/nonempty string fits a newly allocated standard 512-byte block | [x] |
| 45 | `stbds_stralloc` | String fits existing remaining space | [x] |
| 46 | `stbds_stralloc` | String exceeds current block size with empty arena, taking dedicated-block path | [x] |
| 47 | `stbds_stralloc` | String exceeds current block size with existing arena storage, linking dedicated block after head | [x] |
| 48 | `stbds_stralloc` | Repeated exhaustion grows blocks up to 1 MiB and exercises the maximum-size no-increment branch | [x] |
| 49 | `stbds_strreset` | Empty arena versus standard/dedicated multi-block arena; state zeroed after free | [x] |
| 50 | `strkey` | Negative, zero, positive, `INT_MIN`, and `INT_MAX` values | [x] |
| 51 | `hm_geti` | `num <= 0`, so all data loops are skipped | [x] |
| 52 | `hm_geti` | Small positive counts covering odd/even and modulo-4 branches | [x] |
| 53 | `hm_geti` | Larger positive counts crossing map grow, rebuild, and shrink thresholds | [x] |
