# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` defines no
options or conditional sources. There is exactly one valid build-time feature
combination:

| # | Cargo feature combination | CMake configuration | |
|---|---------------------------|---------------------|---|
| 1 | empty set (`--no-default-features --features ''`) | default | [x] |

## Runtime and Input Configurations

Rows are derived from public definitions plus branches, switches, constants,
and size thresholds in `c_src/src/lib.c`. Internal helpers are exercised through
the lowest-level exported entry point that reaches them.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| 1 | `stbds_arrgrowf` | null array; `addlen == 0`; `min_cap == 1..3`, exercising minimum capacity 4 | [x] |
| 2 | `stbds_arrgrowf` | null array; `addlen > min_cap`, so required length selects capacity | [x] |
| 3 | `stbds_arrgrowf` | existing array; requested capacity `<= capacity`, returning the same allocation | [x] |
| 4 | `stbds_arrgrowf` | existing array; requested capacity below `2 * capacity`, doubling capacity | [x] |
| 5 | `stbds_arrgrowf` | existing array; requested capacity at least `2 * capacity`, using exact request | [x] |
| 6 | `stbds_arrgrowf`, `stbds_arrfreef` | element widths 1, 2, 4, 8, and a non-power-of-two struct width; preserve existing bytes across realloc | [x] |
| 7 | `stbds_hash_string` | empty NUL-terminated string; seeds 0, 1, and boundary/random `size_t` values | [x] |
| 8 | `stbds_hash_string` | nonempty ASCII strings of lengths around rotate/wrap boundaries; varied seeds | [x] |
| 9 | `stbds_hash_string` | bytes with the high bit set before NUL, confirming unsigned-char ingestion | [x] |
| 10 | `stbds_hash_bytes` | lengths 0..7 (every tail-switch case), including `(NULL, 0)`; varied seeds | [x] |
| 11 | `stbds_hash_bytes` | exactly one full `size_t` block (8 bytes on this build) | [x] |
| 12 | `stbds_hash_bytes` | multiple full blocks with tail lengths 0..7; randomized bytes and seeds | [x] |
| 13 | `stbds_rand_seed`, `stbds_hmput_key` | seed 0, 1, maximum, and randomized values before constructing a fresh table | [x] |
| 14 | `stbds_hmput_default` | null map creates one zeroed default element | [x] |
| 15 | `stbds_hmput_default` | existing map with default element returns unchanged pointer/content | [x] |
| 16 | `stbds_hmget_key_ts`, `stbds_hmget_key` | null map lookup creates default element and reports index `-1` | [x] |
| 17 | `stbds_hmget_key_ts`, `stbds_hmget_key` | map has default storage but no hash table; lookup reports `-1` | [x] |
| 18 | `stbds_hmput_key` | binary mode (`mode < 1`), new fixed-width keys of widths 1, 2, 4, 8, and 16 | [x] |
| 19 | `stbds_hmput_key` | binary mode, repeated key updates existing slot rather than appending | [x] |
| 20 | `stbds_hmget_key_ts`, `stbds_hmget_key` | binary table, present and absent keys, both probe-bucket scan segments | [x] |
| 21 | `stbds_hmput_key` | binary inserts crossing 6/12/... 75%-load thresholds, causing table growth and rehash | [x] |
| 22 | `stbds_hmput_key` | binary collisions/probe wrap with many randomized keys | [x] |
| 23 | `stbds_hmdel_key` | null map | [x] |
| 24 | `stbds_hmdel_key` | map with default storage but no table | [x] |
| 25 | `stbds_hmdel_key` | initialized binary table, missing key | [x] |
| 26 | `stbds_hmdel_key` | delete binary final element (`old_index == final_index`) | [x] |
| 27 | `stbds_hmdel_key` | delete binary non-final element and repair moved element index | [x] |
| 28 | `stbds_hmdel_key` | insert after deletion reuses a tombstone | [x] |
| 29 | `stbds_hmdel_key` | enough deletions exceed tombstone threshold and rebuild at same slot count | [x] |
| 30 | `stbds_hmdel_key` | enough deletions fall below 25% load and shrink a table larger than 8 slots | [x] |
| 31 | `stbds_shmode_func` | storage mode `STBDS_SH_NONE` (0), then binary-key insertion default switch arm | [x] |
| 32 | `stbds_shmode_func`, string hash APIs | storage mode `STBDS_SH_DEFAULT` (1), borrowing input string pointers | [x] |
| 33 | `stbds_shmode_func`, string hash APIs | storage mode `STBDS_SH_STRDUP` (2), copying and independently freeing strings | [x] |
| 34 | `stbds_shmode_func`, string hash APIs | storage mode `STBDS_SH_ARENA` (3), arena-copying strings | [x] |
| 35 | `stbds_hmput_key` | direct null-map string mode (`mode == 1`) selects default borrowed-string storage | [x] |
| 36 | `stbds_hmput_key` | string modes: empty, short, duplicate-content/different-pointer, and long keys | [x] |
| 37 | `stbds_hmget_key_ts`, `stbds_hmget_key` | string table present/absent lookups and randomized collision/probe shapes | [x] |
| 38 | `stbds_hmdel_key` | string delete missing, final, and non-final keys in default mode | [x] |
| 39 | `stbds_hmdel_key` | string delete in strdup mode frees removed key; moved-key index is repaired | [x] |
| 40 | `stbds_hmdel_key` | string delete in arena mode retains arena allocations until map free | [x] |
| 41 | `stbds_hmfree_func` | null raw array and allocated map with no hash table | [x] |
| 42 | `stbds_hmfree_func` | binary/default/strdup/arena initialized maps, empty and many elements | [x] |
| 43 | `stbds_stralloc` | zeroed arena; empty and short string allocate first 512-byte block | [x] |
| 44 | `stbds_stralloc` | repeated strings fit in remaining current block | [x] |
| 45 | `stbds_stralloc` | exhausted block allocates next block; block size doubles every other allocation up to 1 MiB | [x] |
| 46 | `stbds_stralloc` | string length greater than selected block size, with empty arena | [x] |
| 47 | `stbds_stralloc` | oversized string with existing arena inserts a side block after the head | [x] |
| 48 | `stbds_strreset` | zeroed arena and arena containing normal plus oversized blocks | [x] |
| 49 | `stbds_shmode_func` and mode-taking hash APIs | out-of-range mode integers below 0 and above named range; raw comparison/cast behavior | [x] |
| 50 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX`; repeated calls overwrite one static 256-byte buffer | [x] |
| 51 | `helxo` | input char values NUL, printable, signed/high-bit boundary; duplicate `"jen"` updates value and stdout order/content | [x] |
| 52 | `stbds_arrgrowf` | null array; `addlen == 0`; `min_cap == 0`, returning `NULL` without allocation | [x] |
