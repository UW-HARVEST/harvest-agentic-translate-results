# Configuration surface

Rows are derived from the exported C functions and the `if`/`switch` branches
in `src/lib.c`. Modes are the C values `HM_BINARY=0`, `HM_STRING=1`,
`SH_NONE=0`, `SH_DEFAULT=1`, `SH_STRDUP=2`, and `SH_ARENA=3`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array; `addlen=0`, `min_cap=0`; request is already satisfied and returns `NULL` | [x] |
| 2 | `stbds_arrgrowf` | null array; explicit `min_cap` 1–3 versus 4 or larger | [x] |
| 3 | `stbds_arrgrowf` | existing array; requested capacity already satisfied; pointer and contents unchanged | [x] |
| 4 | `stbds_arrgrowf` | existing array; growth selected by `2 * old_capacity` | [x] |
| 5 | `stbds_arrgrowf` | existing array; `addlen`/`min_cap` exceeds doubling; exact requested capacity | [x] |
| 6 | `stbds_arrgrowf` / `stbds_arrfreef` | element sizes 1, 4, and multi-field struct; zero, one, and many elements | [x] |
| 7 | `stbds_rand_seed` | seeds 0, 1, fixed ordinary value, and `SIZE_MAX` before new map creation | [x] |
| 8 | `stbds_hash_string` | empty NUL-terminated string across boundary seeds | [x] |
| 9 | `stbds_hash_string` | one-byte and many-byte ASCII strings | [x] |
| 10 | `stbds_hash_string` | bytes with the high bit set before the terminating NUL | [x] |
| 11 | `stbds_hash_bytes` | null or non-null data with `len=0` | [x] |
| 12 | `stbds_hash_bytes` | tail lengths 1, 2, 3, 4, 5, 6, and 7 | [x] |
| 13 | `stbds_hash_bytes` | exactly one full `size_t` block (`len=8`) | [x] |
| 14 | `stbds_hash_bytes` | multiple blocks plus every tail remainder (`len=9..23`) | [x] |
| 15 | `stbds_hash_bytes` | high-bit bytes in low word, high word, and tail positions | [x] |
| 16 | `stbds_hmget_key_ts` | null map creates zeroed default element and writes temp `-1` | [x] |
| 17 | `stbds_hmget_key` | null map creates zeroed default element and stores header temp `-1` | [x] |
| 18 | `stbds_hmput_default` | null map creates one zeroed default element | [x] |
| 19 | `stbds_hmput_default` | existing map with default element; no allocation/content change | [x] |
| 20 | `stbds_hmput_key` | null binary map; key sizes 0, 1, 4, 8, and struct-sized | [x] |
| 21 | `stbds_hmput_key` | binary insertion of new key versus update of existing key | [x] |
| 22 | `stbds_hmput_key` | binary insert count below, at, and above 8-slot growth threshold | [x] |
| 23 | `stbds_hmget_key` | binary map hit versus miss, including zero-length key | [x] |
| 24 | `stbds_hmget_key_ts` | binary map hit versus miss; caller temp independent of header temp | [x] |
| 25 | `stbds_hmdel_key` | non-null binary map with no hash table (default-only map) | [x] |
| 26 | `stbds_hmdel_key` | binary missing key leaves map unchanged and temp 0 | [x] |
| 27 | `stbds_hmdel_key` | binary hit deleting final element | [x] |
| 28 | `stbds_hmdel_key` | binary hit deleting non-final element; moved index repaired | [x] |
| 29 | `stbds_hmdel_key` | nonzero `keyoffset` for a binary struct key | [x] |
| 30 | `stbds_hmdel_key` | enough deletes to trigger table shrink | [x] |
| 31 | `stbds_hmdel_key` / `stbds_hmput_key` | tombstone creation followed by tombstone reuse/rebuild | [x] |
| 32 | `stbds_shmode_func` | `SH_DEFAULT`; borrowed string keys | [x] |
| 33 | `stbds_shmode_func` | `SH_STRDUP`; independently allocated string keys | [x] |
| 34 | `stbds_shmode_func` | `SH_ARENA`; arena-copied string keys | [x] |
| 35 | `stbds_shmode_func` | mode values outside 0–3; C truncates to `unsigned char` and does not reject | [x] |
| 36 | `stbds_hmput_key` | null string map implicitly selects `SH_DEFAULT` | [x] |
| 37 | `stbds_hmput_key` | string insertion of new key versus update; temp key behavior | [x] |
| 38 | `stbds_hmput_key` | string map growth across load threshold for default/strdup/arena storage | [x] |
| 39 | `stbds_hmget_key` / `stbds_hmget_key_ts` | string hit and miss for empty, short, long, and high-bit keys | [x] |
| 40 | `stbds_hmdel_key` | string missing key and hit deleting final/non-final entries | [x] |
| 41 | `stbds_hmdel_key` | string deletion under default, strdup, and arena storage | [x] |
| 42 | `stbds_hmfree_func` | binary/default-string/strdup/arena maps, empty and populated | [x] |
| 43 | `stbds_stralloc` | zeroed arena; empty string and short string allocate first 512-byte block | [x] |
| 44 | `stbds_stralloc` | repeated strings fit in current block (`len <= remaining`) | [x] |
| 45 | `stbds_stralloc` | exhausted block allocates the next geometrically sized block | [x] |
| 46 | `stbds_stralloc` | string longer than current block with no existing storage | [x] |
| 47 | `stbds_stralloc` | string longer than current block with existing storage; oversized block linked after head | [x] |
| 48 | `stbds_stralloc` | lengths around 512, 1024, `1<<20`, and one byte beyond each boundary | [x] |
| 49 | `stbds_strreset` | zeroed arena and arena containing one/many/oversized blocks | [x] |
| 50 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` | [x] |
| 51 | `strkey` | sequential calls overwrite and reuse the same 256-byte static buffer | [x] |
| 52 | `sh_puts` | `num < 0`, `num=0`, `num=1`, block-boundary counts, and many allocations; stdout `a <num>\n` | [x] |

Cargo declares no features, so this table has one feature combination. It is
run both normally and with `--no-default-features` to verify that equivalence.
