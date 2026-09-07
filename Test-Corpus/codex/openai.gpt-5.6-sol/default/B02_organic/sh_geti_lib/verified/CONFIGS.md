# Configuration-surface table

Rows are derived from branches and switches in `c_src/src/lib.c`, including all
16 exported entry points. There are no Cargo features; the only build
configurations are default and `--no-default-features`, which are equivalent
but both are verified.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| C01 | `stbds_rand_seed` | seed `0`, ordinary seed, and `SIZE_MAX`; construct a fresh table after each | [x] |
| C02 | `stbds_hash_string` | empty NUL-terminated string | [x] |
| C03 | `stbds_hash_string` | nonempty ASCII strings of varying lengths | [x] |
| C04 | `stbds_hash_string` | nonempty strings containing bytes `0x80..0xff` before NUL | [x] |
| C05 | `stbds_hash_bytes` | zero bytes (null and non-null pointer forms) | [x] |
| C06 | `stbds_hash_bytes` | lengths with each tail size `1..7` | [x] |
| C07 | `stbds_hash_bytes` | one or more full 8-byte words, with and without a tail | [x] |
| C08 | `stbds_arrgrowf` | null array, requested minimum/add length below 4 | [x] |
| C09 | `stbds_arrgrowf` | null array, requested minimum at least 4 | [x] |
| C10 | `stbds_arrgrowf` | existing array with request within capacity (same pointer/state) | [x] |
| C11 | `stbds_arrgrowf` | existing array where doubling capacity wins | [x] |
| C12 | `stbds_arrgrowf` | existing array where requested minimum/add length wins | [x] |
| C13 | `stbds_arrfreef` | free each allocated array shape | [x] |
| C14 | `stbds_stralloc` | first short string creates a 512-byte arena block | [x] |
| C15 | `stbds_stralloc` | repeated short strings reuse remaining block space | [x] |
| C16 | `stbds_stralloc` | string length exactly at/around remaining block boundary | [x] |
| C17 | `stbds_stralloc` | string larger than current block gets a dedicated block | [x] |
| C18 | `stbds_stralloc` | block growth progression up to the 1 MiB cap | [x] |
| C19 | `stbds_strreset` | empty arena and arena containing regular/dedicated blocks | [x] |
| C20 | `stbds_hmput_default` | null map creates zeroed default element | [x] |
| C21 | `stbds_hmput_default` | existing default-only map is unchanged | [x] |
| C22 | `stbds_hmget_key_ts`, `stbds_hmget_key` | null/default-only map lookup | [x] |
| C23 | `stbds_shmode_func` | string ownership mode `STBDS_SH_STRDUP` (`2`) | [x] |
| C24 | `stbds_shmode_func` | string ownership mode `STBDS_SH_ARENA` (`3`) | [x] |
| C25 | `stbds_hmput_key` | binary mode (`0`), insert new scalar key | [x] |
| C26 | `stbds_hmput_key` | binary mode, replace/find existing key | [x] |
| C27 | `stbds_hmput_key` | binary keys with sizes `0`, `1`, `4`, `8`, and non-power-of-two | [x] |
| C28 | `stbds_hmput_key` | string mode (`1`) with default borrowed-key ownership | [x] |
| C29 | `stbds_hmput_key` | string mode with strdup ownership | [x] |
| C30 | `stbds_hmput_key` | string mode with arena ownership | [x] |
| C31 | `stbds_hmput_key` | enough unique keys to grow table and backing array repeatedly | [x] |
| C32 | `stbds_hmput_key` | insert after deletion reuses a tombstone | [x] |
| C33 | `stbds_hmget_key_ts`, `stbds_hmget_key` | present and absent binary keys in populated table | [x] |
| C34 | `stbds_hmget_key_ts`, `stbds_hmget_key` | present and absent string keys in each ownership mode | [x] |
| C35 | `stbds_hmdel_key` | null map, default-only map, and absent key | [x] |
| C36 | `stbds_hmdel_key` | delete last element | [x] |
| C37 | `stbds_hmdel_key` | delete non-last element and repair moved index | [x] |
| C38 | `stbds_hmdel_key` | deletions trigger table shrink | [x] |
| C39 | `stbds_hmdel_key` | deletions trigger same-size tombstone rebuild | [x] |
| C40 | `stbds_hmfree_func` | null, binary, borrowed-string, strdup, and arena maps | [x] |
| C41 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` | [x] |
| C42 | `sh_geti` | negative and zero counts (empty loops/maps) | [x] |
| C43 | `sh_geti` | one element | [x] |
| C44 | `sh_geti` | many elements spanning hash growth/deletion paths; compare stdout bytes | [x] |
| C45 | all hash-map entry points | out-of-range mode `-1` (binary branch) | [x] |
| C46 | all hash-map entry points | out-of-range mode `4` (string branch plus default storage switch) | [x] |
