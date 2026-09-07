# Configuration surface

Rows are derived from public exports plus the `if`/`switch` branches, constants,
and data-shape distinctions in `c_src/src/lib.c`. “Binary” means `mode < 1`;
“string” means `mode >= 1`, matching the C comparisons rather than assuming a
closed enum.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | length `0`, arbitrary seed | [x] |
| 2 | `stbds_hash_bytes` | tail length `1` (switch case 1) | [x] |
| 3 | `stbds_hash_bytes` | tail length `2` (switch case 2) | [x] |
| 4 | `stbds_hash_bytes` | tail length `3` (switch case 3) | [x] |
| 5 | `stbds_hash_bytes` | tail length `4` (switch case 4, including high-bit byte) | [x] |
| 6 | `stbds_hash_bytes` | tail length `5` (switch case 5) | [x] |
| 7 | `stbds_hash_bytes` | tail length `6` (switch case 6) | [x] |
| 8 | `stbds_hash_bytes` | tail length `7` (switch case 7) | [x] |
| 9 | `stbds_hash_bytes` | one or more full 8-byte words, no tail | [x] |
| 10 | `stbds_hash_bytes` | multiple full words plus each tail remainder `1..7` | [x] |
| 11 | `stbds_hash_string` | empty NUL-terminated string | [x] |
| 12 | `stbds_hash_string` | one/many ASCII bytes | [x] |
| 13 | `stbds_hash_string` | bytes with high bit set before NUL | [x] |
| 14 | `stbds_rand_seed`, map APIs | repeated identical seed before fresh maps | [x] |
| 15 | `stbds_rand_seed`, map APIs | different seeds before fresh maps | [x] |
| 16 | `stbds_arrgrowf` | null array, `addlen=0`, `min_cap=0` (no allocation) | [x] |
| 17 | `stbds_arrgrowf` | null array, positive `addlen`, `min_cap=0` (minimum capacity 4) | [x] |
| 18 | `stbds_arrgrowf` | null array, `addlen=0`, positive `min_cap < 4` | [x] |
| 19 | `stbds_arrgrowf` | null array, explicit `min_cap >= 4` | [x] |
| 20 | `stbds_arrgrowf` | existing array, requested minimum at/below capacity (same pointer) | [x] |
| 21 | `stbds_arrgrowf` | existing array, required length forces doubling | [x] |
| 22 | `stbds_arrgrowf` | existing array, explicit minimum at least twice capacity | [x] |
| 23 | `stbds_arrgrowf` | element sizes `1`, `4`, and odd/non-power-of-two size | [x] |
| 24 | `stbds_arrfreef` | free each allocated array shape | [x] |
| 25 | `stbds_hmput_default` | null map creates default-only map | [x] |
| 26 | `stbds_hmput_default` | map already has default slot; repeated call is unchanged | [x] |
| 27 | `stbds_hmget_key_ts` | null map, binary mode, missing key | [x] |
| 28 | `stbds_hmget_key` | default-only map with no hash table, binary missing key | [x] |
| 29 | `stbds_hmget_key`, `stbds_hmget_key_ts` | populated binary map, present key | [x] |
| 30 | `stbds_hmget_key`, `stbds_hmget_key_ts` | populated binary map, absent key reaching first probe segment empty slot | [x] |
| 31 | `stbds_hmget_key`, `stbds_hmget_key_ts` | populated binary map, absent key requiring wrapped probe segment | [x] |
| 32 | `stbds_hmput_key` | binary mode, insert first key into null map | [x] |
| 33 | `stbds_hmput_key` | binary mode, update existing key | [x] |
| 34 | `stbds_hmput_key` | binary mode, enough unique keys to grow table and array repeatedly | [x] |
| 35 | `stbds_hmput_key` | binary mode, key sizes `1`, `4`, `8`, and odd size | [x] |
| 36 | `stbds_hmput_key` | binary element is key-only versus key plus payload | [x] |
| 37 | `stbds_hmdel_key` | null map | [x] |
| 38 | `stbds_hmdel_key` | default-only map with no table | [x] |
| 39 | `stbds_hmdel_key` | populated binary map, absent key | [x] |
| 40 | `stbds_hmdel_key` | delete final array entry (no compaction move) | [x] |
| 41 | `stbds_hmdel_key` | delete non-final entry (move final entry and repair bucket index) | [x] |
| 42 | `stbds_hmdel_key` | nonzero `keyoffset` within an element | [x] |
| 43 | `stbds_hmdel_key`, `stbds_hmput_key` | deletion creates tombstone and later insertion reuses it | [x] |
| 44 | `stbds_hmdel_key` | deletions cross shrink threshold with slot count above 8 | [x] |
| 45 | `stbds_hmdel_key` | tombstones cross rebuild threshold without shrinking | [x] |
| 46 | `stbds_shmode_func` | mode `STBDS_SH_NONE` (`0`) then binary-key operations | [x] |
| 47 | `stbds_shmode_func` | mode `STBDS_SH_DEFAULT` (`1`), borrowed string keys | [x] |
| 48 | `stbds_shmode_func` | mode `STBDS_SH_STRDUP` (`2`), duplicated string keys | [x] |
| 49 | `stbds_shmode_func` | mode `STBDS_SH_ARENA` (`3`), arena-owned string keys | [x] |
| 50 | `stbds_shmode_func` | negative mode (unsigned-char conversion), followed by matching binary-mode calls | [x] |
| 51 | `stbds_shmode_func` | mode above `3`, followed by matching string-mode calls and switch default storage | [x] |
| 52 | string map APIs | empty string key | [x] |
| 53 | string map APIs | one/many keys, present and absent lookup | [x] |
| 54 | string map APIs | update existing key and verify stable stored key semantics | [x] |
| 55 | string map APIs | delete last and non-last entries under borrowed/strdup/arena modes | [x] |
| 56 | `stbds_hmfree_func` | binary map, string borrowed map, strdup map, and arena map | [x] |
| 57 | `stbds_stralloc` | empty string into empty arena | [x] |
| 58 | `stbds_stralloc` | many small strings fit in one 512-byte block | [x] |
| 59 | `stbds_stralloc` | request exactly remaining bytes | [x] |
| 60 | `stbds_stralloc` | request one byte beyond remaining, causing next normal block | [x] |
| 61 | `stbds_stralloc` | request larger than current normal block, causing dedicated block | [x] |
| 62 | `stbds_stralloc` | repeated growth advances block sequence up to 1 MiB cap | [x] |
| 63 | `stbds_strreset` | empty arena | [x] |
| 64 | `stbds_strreset` | arena containing normal and dedicated blocks | [x] |
| 65 | `strkey` | negative integer, including `INT_MIN` | [x] |
| 66 | `strkey` | zero | [x] |
| 67 | `strkey` | positive integer, including `INT_MAX` | [x] |
| 68 | `hm_geti` | negative and zero `num` (loops skipped as dictated by C comparisons) | [x] |
| 69 | `hm_geti` | `num=1`, small many, and values crossing initial capacities | [x] |
| 70 | all map APIs | randomized operation sequences mixing insert/update/get/delete/default/free | [x] |
