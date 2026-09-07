# Configuration-surface table

There are no Cargo features and no conditional-compilation branches in the C source, so the feature matrix is the single default/no-feature configuration. Each row below is derived from a distinct branch, mode, input shape, or public entry point in `src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_rand_seed` | seed `0`, ordinary nonzero seeds, and `SIZE_MAX`; verify subsequent map/hash-index behavior | [x] |
| 2 | `stbds_hash_string` | empty C string | [x] |
| 3 | `stbds_hash_string` | one-byte and multi-byte ASCII strings | [x] |
| 4 | `stbds_hash_string` | bytes with the high bit set before the terminating NUL | [x] |
| 5 | `stbds_hash_bytes` | no full word and tail remainder `0` (`len = 0`) | [x] |
| 6 | `stbds_hash_bytes` | no full word and tail remainder `1` (`len = 1`) | [x] |
| 7 | `stbds_hash_bytes` | no full word and tail remainder `2` (`len = 2`) | [x] |
| 8 | `stbds_hash_bytes` | no full word and tail remainder `3` (`len = 3`) | [x] |
| 9 | `stbds_hash_bytes` | no full word and tail remainder `4` (`len = 4`) | [x] |
| 10 | `stbds_hash_bytes` | no full word and tail remainder `5` (`len = 5`) | [x] |
| 11 | `stbds_hash_bytes` | no full word and tail remainder `6` (`len = 6`) | [x] |
| 12 | `stbds_hash_bytes` | no full word and tail remainder `7` (`len = 7`) | [x] |
| 13 | `stbds_hash_bytes` | one full 64-bit word and tail remainders `0..7` (`len = 8..15`) | [x] |
| 14 | `stbds_hash_bytes` | multiple full words plus every tail remainder (`len >= 16`) | [x] |
| 15 | `stbds_arrgrowf` | null array, requested/add lengths `0..3`; minimum capacity floor becomes `4` when growth is needed | [x] |
| 16 | `stbds_arrgrowf` | null array with explicit minimum capacity `>= 4`; element sizes `1`, `4`, and a padded struct | [x] |
| 17 | `stbds_arrgrowf` | existing array and requested capacity `<= capacity`; return unchanged | [x] |
| 18 | `stbds_arrgrowf` | existing array where requested minimum is below `2 * old_capacity`; capacity doubles | [x] |
| 19 | `stbds_arrgrowf` | existing array where requested minimum is at least `2 * old_capacity`; exact requested/add-driven capacity | [x] |
| 20 | `stbds_arrfreef` | free arrays from rows 15–19 after writing randomized payload bytes | [x] |
| 21 | `stbds_hmput_default` | null map, preallocated zero-length map, and map already containing the default entry | [x] |
| 22 | `stbds_hmget_key_ts` | null map, map with default only/no table, missing key, and present key; inspect returned pointer and `temp` | [x] |
| 23 | `stbds_hmget_key` | same four states as row 22; inspect header `temp` | [x] |
| 24 | `stbds_hmput_key` | binary mode `0`, key sizes `1`, `4`, `8`, and a padded byte key; insert new and replace duplicate | [x] |
| 25 | `stbds_hmput_key` | string mode `1` with default pointer ownership; empty, short, long, and high-byte keys | [x] |
| 26 | `stbds_shmode_func` / `stbds_hmput_key` | string mode `STBDS_SH_STRDUP (2)`; caller mutates/frees source after insertion | [x] |
| 27 | `stbds_shmode_func` / `stbds_hmput_key` | string mode `STBDS_SH_ARENA (3)`; many strings cross arena block boundaries | [x] |
| 28 | `stbds_hmput_key` | enough insertions to cross the 75% used threshold and rehash from 8 to larger slot counts | [x] |
| 29 | `stbds_hmdel_key` | null map, default-only/no-table map, and missing key | [x] |
| 30 | `stbds_hmdel_key` | delete final entry and non-final entry (move-last path), in binary and string modes | [x] |
| 31 | `stbds_hmdel_key` | deletions trigger tombstone rebuild and used-count shrink; include strdup key release | [x] |
| 32 | `stbds_hmfree_func` | null, binary map, default string map, strdup map, and arena map | [x] |
| 33 | `stbds_shmode_func` | modes none `0`, default `1`, strdup `2`, arena `3`; verify initial default entry/header state | [x] |
| 34 | `stbds_stralloc` | fresh arena and strings below/equal to the `512`-byte initial block boundary | [x] |
| 35 | `stbds_stralloc` | repeated allocations while space remains and while block growth advances toward `1 << 20` | [x] |
| 36 | `stbds_stralloc` | string larger than current block, both with empty and nonempty arena storage | [x] |
| 37 | `stbds_strreset` | empty arena, one block, multiple blocks, and dedicated oversize blocks | [x] |
| 38 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` inputs | [x] |
| 39 | `helxo` | ordinary printable letters, bytes with high bit set, and NUL; capture and compare stdout bytes | [x] |

Feature combinations:

- default features (there are no declared features)
- `--no-default-features` (same source configuration, retained as a completion-gate build)
