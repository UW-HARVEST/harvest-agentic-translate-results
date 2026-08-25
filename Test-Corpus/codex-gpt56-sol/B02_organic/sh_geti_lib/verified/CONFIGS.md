# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and CMake defines no options or
conditional sources. The full valid build-time feature set is therefore:

| # | Cargo invocation | CMake configuration | [x] |
|---|------------------|---------------------|-----|
| B1 | `--no-default-features` (empty feature set) | default | [x] |

## Runtime Configurations

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array; requested capacity 0 returns null, while 1-3 exercise minimum capacity 4 | [x] |
| 2 | `stbds_arrgrowf` | null array; requested capacity at least 4; element sizes 1, 4, and 16 | [x] |
| 3 | `stbds_arrgrowf` | existing array; requested minimum/addition fits current capacity (same pointer/data) | [x] |
| 4 | `stbds_arrgrowf` | existing array; required length exceeds requested minimum | [x] |
| 5 | `stbds_arrgrowf` | existing array; growth below twice capacity (doubling branch) | [x] |
| 6 | `stbds_arrgrowf`, `stbds_arrfreef` | existing array; explicit minimum at/above twice capacity; preserve bytes and free | [x] |
| 7 | `stbds_rand_seed` | seeds 0, 1, default seed, high-bit values, and `SIZE_MAX` before new tables | [x] |
| 8 | `stbds_hash_string` | empty C string across boundary/random seeds | [x] |
| 9 | `stbds_hash_string` | one-byte strings, including bytes with the high bit set | [x] |
| 10 | `stbds_hash_string` | randomized multi-byte C strings and lengths | [x] |
| 11 | `stbds_hash_bytes` | zero length with a null data pointer | [x] |
| 12 | `stbds_hash_bytes` | tail lengths 1 through 7 (every switch arm), randomized bytes/seeds | [x] |
| 13 | `stbds_hash_bytes` | exact 8-byte blocks, including high-bit bytes | [x] |
| 14 | `stbds_hash_bytes` | one/many blocks plus tails 0 through 7; randomized lengths through 256 | [x] |
| 15 | `stbds_hmput_default` | null map creates one zeroed default entry | [x] |
| 16 | `stbds_hmput_default` | existing default entry is retained unchanged | [x] |
| 17 | `stbds_hmget_key`, `stbds_hmget_key_ts` | null/existing table-less map and absent key | [x] |
| 18 | `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts` | binary keys with key sizes 1, 4, 8, and 16; insert then found lookup | [x] |
| 19 | `stbds_hmput_key` | binary existing-key replacement path; item count/order unchanged | [x] |
| 20 | `stbds_hmput_key` | binary randomized many-entry insertion crossing 8-slot growth thresholds | [x] |
| 21 | `stbds_hmdel_key` | null, table-less, and populated map with missing binary key | [x] |
| 22 | `stbds_hmdel_key` | delete final binary entry (no move) | [x] |
| 23 | `stbds_hmdel_key` | delete non-final binary entry (move and slot-index repair) | [x] |
| 24 | `stbds_hmdel_key`, `stbds_hmput_key` | reuse a tombstone on later insertion | [x] |
| 25 | `stbds_hmdel_key` | many deletions trigger tombstone rebuild | [x] |
| 26 | `stbds_hmdel_key` | many deletions trigger table shrink | [x] |
| 27 | `stbds_shmode_func` | modes `STBDS_SH_NONE`/`DEFAULT`/`STRDUP`/`ARENA` and out-of-range `int` values | [x] |
| 28 | string map low-level API | default/reference string mode: empty, one, and many keys | [x] |
| 29 | string map low-level API | strdup mode: insert/update/get/delete/free with source buffers reused | [x] |
| 30 | string map low-level API | arena mode: insert/update/get/delete/free across arena blocks | [x] |
| 31 | string map low-level API | string lookups in populated tables: present and absent keys | [x] |
| 32 | `stbds_hmfree_func` | binary map, table-less map, each string ownership mode, and null | [x] |
| 33 | `stbds_stralloc` | empty/short strings fitting a new or existing 512-byte block | [x] |
| 34 | `stbds_stralloc` | string larger than current block with no prior storage | [x] |
| 35 | `stbds_stralloc` | oversized string with prior storage (side-block insertion branch) | [x] |
| 36 | `stbds_stralloc` | repeated allocations grow block exponent up to 1 MiB cap | [x] |
| 37 | `stbds_strreset` | empty and populated arena; all fields reset to zero | [x] |
| 38 | `strkey` | zero, positive, negative, `INT_MIN`, and `INT_MAX` | [x] |
| 39 | `sh_geti` | negative/zero, one, odd/even small counts, and randomized larger counts; both strdup and arena passes | [x] |
