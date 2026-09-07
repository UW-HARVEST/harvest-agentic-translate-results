# Configuration surface

There are no Cargo features and no C compile-time feature switches in this
crate. The rows enumerate the runtime modes and input shapes selected by the C
branches for every public dynamic symbol.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_rand_seed` | seed `0`, ordinary nonzero seed, and `SIZE_MAX`; verify effect on newly created tables | [x] |
| 2 | `stbds_hash_string` | empty NUL-terminated string across varied seeds | [x] |
| 3 | `stbds_hash_string` | one-byte and multi-byte ASCII strings across varied seeds | [x] |
| 4 | `stbds_hash_string` | bytes with the high bit set (C casts each byte to `unsigned char`) | [x] |
| 5 | `stbds_hash_bytes` | `len == 0`, including `p == NULL` | [x] |
| 6 | `stbds_hash_bytes` | tail lengths `1..=3` | [x] |
| 7 | `stbds_hash_bytes` | tail lengths `4..=7`, including high-bit byte at tail offset 3 | [x] |
| 8 | `stbds_hash_bytes` | exactly one full `size_t` block (`len == 8` on this build) | [x] |
| 9 | `stbds_hash_bytes` | multiple full blocks and full-block-plus-tail lengths | [x] |
| 10 | `stbds_arrgrowf` | null array with zero request returns `NULL`; a nonzero requested capacity/add length below 4 allocates capacity 4 | [x] |
| 11 | `stbds_arrgrowf` | null array with explicit minimum capacity at/above 4 | [x] |
| 12 | `stbds_arrgrowf` | existing array and requested minimum/added length already within capacity; pointer unchanged | [x] |
| 13 | `stbds_arrgrowf` | existing array grows by doubling old capacity | [x] |
| 14 | `stbds_arrgrowf` | existing array grows directly to a requested minimum larger than double | [x] |
| 15 | `stbds_arrgrowf` / `stbds_arrfreef` | varied element widths/alignment (`1`, `4`, `8`, struct width), then free | [x] |
| 16 | `stbds_hmput_default` | null map creates one zeroed default slot | [x] |
| 17 | `stbds_hmput_default` | already initialized map returns unchanged and preserves default slot | [x] |
| 18 | `stbds_hmget_key_ts` | null binary map lookup creates default slot and reports index `-1` | [x] |
| 19 | `stbds_hmget_key_ts` | map has default slot but no hash table | [x] |
| 20 | `stbds_hmget_key_ts` | existing binary key found versus missing, using explicit `temp` | [x] |
| 21 | `stbds_hmget_key` | existing binary key found versus missing, result stored in header `temp` | [x] |
| 22 | `stbds_hmput_key` | binary mode (`mode < 1`), new key and duplicate-key update path | [x] |
| 23 | `stbds_hmput_key` | binary keys of widths `1`, `4`, `8`, and multi-field struct width | [x] |
| 24 | `stbds_hmput_key` | enough binary keys to cross 8-slot load threshold and rehash to larger tables | [x] |
| 25 | `stbds_hmput_key` | hashes below reserved value 2 are adjusted (empty string with table seed `0` and `1`) | [x] |
| 26 | `stbds_shmode_func` / `stbds_hmput_key` | string mode `STBDS_SH_DEFAULT` (`1`): borrowed key pointer | [x] |
| 27 | `stbds_shmode_func` / `stbds_hmput_key` | string mode `STBDS_SH_STRDUP` (`2`): duplicated key pointer | [x] |
| 28 | `stbds_shmode_func` / `stbds_hmput_key` | string mode `STBDS_SH_ARENA` (`3`): arena-owned key pointer | [x] |
| 29 | `stbds_shmode_func` / `stbds_hmput_key` | mode `0` uses binary key copy despite construction through `shmode_func` | [x] |
| 30 | `stbds_shmode_func` / map APIs | out-of-range negative mode (binary branch) and mode `>= 4` (string branch, stored as `unsigned char`) | [x] |
| 31 | string map APIs | empty, one-byte, long, shared-prefix, and high-bit-byte string keys; found/missing/duplicate | [x] |
| 32 | `stbds_hmdel_key` | null map and no-table map no-op paths | [x] |
| 33 | `stbds_hmdel_key` | missing key in populated binary and string maps | [x] |
| 34 | `stbds_hmdel_key` | delete the final entry (no compaction move) | [x] |
| 35 | `stbds_hmdel_key` | delete a non-final entry; moved final entry's index is repaired | [x] |
| 36 | `stbds_hmdel_key` | nonzero `keyoffset` with a binary struct key | [x] |
| 37 | `stbds_hmdel_key` | enough deletions after growth to trigger table shrinking | [x] |
| 38 | `stbds_hmdel_key` | enough deletions to cross tombstone threshold and rebuild at the same size | [x] |
| 39 | `stbds_hmdel_key` | string deletion in borrowed, strdup, and arena ownership modes | [x] |
| 40 | `stbds_hmfree_func` | populated binary map; populated string maps in all ownership modes | [x] |
| 41 | `stbds_stralloc` | zeroed arena, short string (`len <= 512`) allocates first normal block | [x] |
| 42 | `stbds_stralloc` | repeated short strings fit in remaining space and are packed from the block end | [x] |
| 43 | `stbds_stralloc` | normal-block exhaustion allocates progressively larger blocks | [x] |
| 44 | `stbds_stralloc` | string length exceeds current block size; dedicated oversized block with empty arena | [x] |
| 45 | `stbds_stralloc` | oversized string when normal storage already exists; dedicated block is linked after the head | [x] |
| 46 | `stbds_stralloc` | block growth reaches the `1 << 20` maximum and stops incrementing `block` | [x] |
| 47 | `stbds_strreset` | empty arena and populated/multi-block arena; all fields reset to zero | [x] |
| 48 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` formatting | [x] |
| 49 | `str_dups` | `num <= 0`: loop skipped, one strdup map entry printed | [x] |
| 50 | `str_dups` | small positive `num`: arena loop runs, reset, then exact output | [x] |
| 51 | `str_dups` | larger `num` crossing arena block boundaries; exact output | [x] |
