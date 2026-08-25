# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options or preprocessor definitions. There is exactly one valid combination:

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|--------|
| 1 | `--no-default-features` (empty feature set) | default configuration | [x] |

## Runtime and Input Configurations

Modes are mechanically derived from `mode >= STBDS_HM_STRING`, the
`table->string.mode` switch, and the public low-level entry points. Shapes are
derived from the hash tail switch, array growth branches, hash load thresholds,
delete/rebuild thresholds, and string-arena min/max constants.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `stbds_rand_seed` | seeds `0`, ordinary nonzero values, and `SIZE_MAX`, followed by fresh binary and string maps | [x] |
| 2 | `stbds_hash_string` | empty NUL-terminated string; varied seeds | [x] |
| 3 | `stbds_hash_string` | one-byte and many-byte ASCII strings; varied seeds | [x] |
| 4 | `stbds_hash_string` | non-ASCII bytes with the high bit set and no interior NUL | [x] |
| 5 | `stbds_hash_bytes` | length `0` (tail switch case 0), including `p == NULL` | [x] |
| 6 | `stbds_hash_bytes` | length `1` (tail switch case 1) | [x] |
| 7 | `stbds_hash_bytes` | length `2` (tail switch case 2) | [x] |
| 8 | `stbds_hash_bytes` | length `3` (tail switch case 3) | [x] |
| 9 | `stbds_hash_bytes` | length `4` (tail switch case 4 and signed `int` promotion) | [x] |
| 10 | `stbds_hash_bytes` | length `5` (tail switch case 5) | [x] |
| 11 | `stbds_hash_bytes` | length `6` (tail switch case 6) | [x] |
| 12 | `stbds_hash_bytes` | length `7` (tail switch case 7) | [x] |
| 13 | `stbds_hash_bytes` | one or more full `sizeof(size_t)` words with remainder 0-7 | [x] |
| 14 | `stbds_hash_bytes` | full words and tails containing high-bit bytes | [x] |
| 15 | `stbds_arrgrowf` | null array, `addlen == 0`, `min_cap == 0`; no-growth branch returns `NULL` before the capacity floor | [x] |
| 16 | `stbds_arrgrowf` | null array where `addlen > min_cap`, and where `min_cap > addlen` | [x] |
| 17 | `stbds_arrgrowf` | existing array request at or below capacity; pointer/capacity unchanged | [x] |
| 18 | `stbds_arrgrowf` | existing array request below twice capacity; geometric doubling | [x] |
| 19 | `stbds_arrgrowf` | existing array request at or above twice capacity; direct requested capacity | [x] |
| 20 | `stbds_arrfreef` | free arrays produced by each growth path | [x] |
| 21 | `stbds_hmput_default` | null map versus existing map with default entry | [x] |
| 22 | `stbds_hmget_key`, `stbds_hmget_key_ts` | binary mode, null/no-table map, absent key, and present key | [x] |
| 23 | `stbds_hmput_key` | binary mode insert and update for key sizes 1, 2, 4, 8, and non-power-of-two | [x] |
| 24 | `stbds_hmput_key` | binary mode one, many, and enough entries to grow the hash index | [x] |
| 25 | `stbds_hmdel_key` | binary mode null/no-table map, missing key, final element, and non-final element | [x] |
| 26 | `stbds_hmdel_key`, `stbds_hmput_key` | binary deletion tombstone reused by a later insertion | [x] |
| 27 | `stbds_hmdel_key` | binary delete volume crossing tombstone rebuild threshold | [x] |
| 28 | `stbds_hmdel_key` | binary delete volume crossing used-count shrink threshold | [x] |
| 29 | `stbds_shmode_func`, `stbds_hmput_key` | `STBDS_SH_DEFAULT`; empty, one, duplicate, and many string keys | [x] |
| 30 | `stbds_shmode_func`, `stbds_hmput_key` | `STBDS_SH_STRDUP`; source strings mutated/freed after insertion | [x] |
| 31 | `stbds_shmode_func`, `stbds_hmput_key` | `STBDS_SH_ARENA`; empty, small, repeated, and large string keys | [x] |
| 32 | `stbds_hmget_key`, `stbds_hmget_key_ts` | string mode present and absent keys after index growth | [x] |
| 33 | `stbds_hmdel_key` | string mode missing, final, and non-final keys under default/strdup/arena storage | [x] |
| 34 | `stbds_hmfree_func` | null, binary map, and all three string ownership modes | [x] |
| 35 | hash-map low-level entry points | mode `0`, negative mode, threshold mode `1`, and out-of-range positive modes | [x] |
| 36 | `stbds_stralloc` | zeroed arena and empty string | [x] |
| 37 | `stbds_stralloc` | small strings that fit, exactly fill, and exhaust a normal block | [x] |
| 38 | `stbds_stralloc` | repeated allocations causing normal block-size growth | [x] |
| 39 | `stbds_stralloc` | string length just above current block size (dedicated block path) | [x] |
| 40 | `stbds_stralloc` | allocations around 512-byte minimum and 1 MiB maximum constants | [x] |
| 41 | `stbds_strreset` | zeroed arena and populated multi-block arena | [x] |
| 42 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` | [x] |
| 43 | `intput` | randomized integers excluding duplicate-key values `9` and `11` | [x] |
