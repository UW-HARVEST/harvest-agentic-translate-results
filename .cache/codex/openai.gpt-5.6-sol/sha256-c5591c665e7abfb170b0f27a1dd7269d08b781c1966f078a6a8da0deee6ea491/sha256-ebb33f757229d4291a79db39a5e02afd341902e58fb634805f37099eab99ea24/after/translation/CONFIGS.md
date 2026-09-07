# Configuration Surface

Derived from the exported functions plus every option, mode, size, threshold,
switch arm, and input-shape branch in `c_src/src/lib.c`. Cargo declares no
features, so the only build configuration is the same code under default and
`--no-default-features`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array with zero add/min returns `NULL`; requesting one element takes floor growth to capacity 4; randomized element sizes | [x] |
| 2 | `stbds_arrgrowf` | null array; `addlen > min_cap` so required length controls capacity | [x] |
| 3 | `stbds_arrgrowf` | existing array; requested capacity already available, pointer/capacity unchanged | [x] |
| 4 | `stbds_arrgrowf` | existing array; growth below twice current capacity, so capacity doubles | [x] |
| 5 | `stbds_arrgrowf` / `stbds_arrfreef` | existing array; explicit larger minimum wins; payload survives reallocation; then free | [x] |
| 6 | `stbds_rand_seed` | zero, fixed, and randomized seeds followed by fresh hash-table creation | [x] |
| 7 | `stbds_hash_string` | empty C string across randomized seeds | [x] |
| 8 | `stbds_hash_string` | one/many ASCII bytes, high-bit bytes, and randomized NUL-terminated strings | [x] |
| 9 | `stbds_hash_bytes` | lengths 0 through 7, covering every tail `switch` arm, across randomized seeds/data | [x] |
| 10 | `stbds_hash_bytes` | lengths 8, 9, 15, 16, and many blocks, covering full-word loop plus tails | [x] |
| 11 | `stbds_hmput_default` | null map and already initialized map; default element zeroed and retained | [x] |
| 12 | `stbds_hmget_key_ts` | null map creates default element and returns index sentinel | [x] |
| 13 | `stbds_hmget_key` | wrapper path on null/empty map stores result in header temp | [x] |
| 14 | `stbds_hmput_key` / get APIs | binary mode (`mode < 1`), scalar and multi-byte keys, new and existing keys | [x] |
| 15 | `stbds_hmput_key` / get APIs | string mode (`mode >= 1`), empty/short/long keys, new and existing keys | [x] |
| 16 | `stbds_hmput_key` / get APIs | enough distinct keys to cross 8-slot load threshold and repeatedly resize | [x] |
| 17 | `stbds_shmode_func` / map APIs | string mode `STBDS_SH_NONE` (0) with binary-key behavior | [x] |
| 18 | `stbds_shmode_func` / map APIs | `STBDS_SH_DEFAULT` (1), borrowed string-key pointer retained | [x] |
| 19 | `stbds_shmode_func` / map APIs | `STBDS_SH_STRDUP` (2), string key copied and owned | [x] |
| 20 | `stbds_shmode_func` / map APIs | `STBDS_SH_ARENA` (3), string key copied into arena | [x] |
| 21 | `stbds_hmdel_key` | null map, map without table, initialized map with absent key | [x] |
| 22 | `stbds_hmdel_key` | delete present first/middle/last binary key, including compacting the final element | [x] |
| 23 | `stbds_hmdel_key` | delete present string key under default/strdup/arena ownership | [x] |
| 24 | `stbds_hmdel_key` | repeated insert/delete crossing shrink threshold and tombstone rebuild threshold | [x] |
| 25 | `stbds_hmfree_func` | null, binary map, and string maps in default/strdup/arena ownership modes | [x] |
| 26 | `stbds_stralloc` | empty and short strings fitting in a new 512-byte arena block | [x] |
| 27 | `stbds_stralloc` | repeated strings fitting remaining space, then forcing geometrically larger blocks | [x] |
| 28 | `stbds_stralloc` | string larger than selected block, taking dedicated-block insertion branch | [x] |
| 29 | `stbds_stralloc` | block growth at and above the 1 MiB cap | [x] |
| 30 | `stbds_strreset` | zeroed arena, one block, multiple blocks, and dedicated oversize blocks | [x] |
| 31 | `strkey` | negative, zero, positive, and `int` boundary values; returned static-buffer bytes | [x] |
| 32 | `str_put` | negative/zero/one/many loop counts; stdout bytes and internal string-map path | [x] |
| 33 | all exports | release build with Cargo default feature set (empty) | [x] |
| 34 | all exports | release/test build with `--no-default-features` (same empty feature set) | [x] |
