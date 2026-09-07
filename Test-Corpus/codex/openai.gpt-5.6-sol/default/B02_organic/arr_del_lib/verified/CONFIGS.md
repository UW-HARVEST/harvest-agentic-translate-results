# Configuration surface

The crate has no Cargo features and neither build defines an executable target.
Thus there is one build configuration: the library with its unconditional C/Rust
code. Rows are derived from the public exports plus the `if`/`switch` branches,
size thresholds, modes, and input shapes in `src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array; element sizes 1, 4, and 16; requested length/capacity 0 or 1, forcing minimum capacity 4 | [x] |
| 2 | `stbds_arrgrowf` | existing array; requested capacity already satisfied, returning the same allocation | [x] |
| 3 | `stbds_arrgrowf` | existing array; required length exceeds capacity and selects capacity doubling | [x] |
| 4 | `stbds_arrgrowf` | existing array; explicit `min_cap` exceeds doubled capacity and is used directly | [x] |
| 5 | `stbds_arrfreef` | free each non-null array shape produced by rows 1-4 | [x] |
| 6 | `stbds_rand_seed` | zero, fixed nonzero, and randomized seeds before new hash-table creation | [x] |
| 7 | `stbds_hash_string` | empty string across many seeds | [x] |
| 8 | `stbds_hash_string` | one-byte and multi-byte ASCII strings across many seeds | [x] |
| 9 | `stbds_hash_string` | bytes with the high bit set before the terminating NUL | [x] |
| 10 | `stbds_hash_bytes` | length 0 with null and non-null pointers | [x] |
| 11 | `stbds_hash_bytes` | tail lengths 1 through 7, covering every switch arm | [x] |
| 12 | `stbds_hash_bytes` | exactly one machine word (8 bytes) | [x] |
| 13 | `stbds_hash_bytes` | multiple full words plus every tail length, including high-bit bytes | [x] |
| 14 | `stbds_hmput_default` | null map creates one zeroed default record | [x] |
| 15 | `stbds_hmput_default` | existing map with default record returns unchanged | [x] |
| 16 | `stbds_hmget_key_ts` | null map creates default record and reports missing key | [x] |
| 17 | `stbds_hmget_key_ts` | backing array with no hash table reports missing key | [x] |
| 18 | `stbds_hmget_key_ts` | binary map hit and miss, including randomized key sizes/values | [x] |
| 19 | `stbds_hmget_key_ts` | string map hit and miss in default, strdup, and arena modes | [x] |
| 20 | `stbds_hmget_key` | binary and string hit/miss; result stored in header `temp` | [x] |
| 21 | `stbds_hmput_key` | binary mode; first insertion into null map | [x] |
| 22 | `stbds_hmput_key` | binary mode; update an existing key without increasing length | [x] |
| 23 | `stbds_hmput_key` | binary mode; enough unique keys to cross 75% load and grow/rehash repeatedly | [x] |
| 24 | `stbds_hmput_key` | collision/probe wrap-around with randomized binary keys and fixed seeds | [x] |
| 25 | `stbds_hmput_key` | string default mode stores caller key pointers | [x] |
| 26 | `stbds_hmput_key` | string strdup mode duplicates keys | [x] |
| 27 | `stbds_hmput_key` | string arena mode allocates keys from arena blocks | [x] |
| 28 | `stbds_shmode_func` | modes NONE(0), DEFAULT(1), STRDUP(2), and ARENA(3) | [x] |
| 29 | `stbds_hmdel_key` | null map | [x] |
| 30 | `stbds_hmdel_key` | backing array has no hash table | [x] |
| 31 | `stbds_hmdel_key` | missing key in a populated binary/string map | [x] |
| 32 | `stbds_hmdel_key` | delete final entry | [x] |
| 33 | `stbds_hmdel_key` | delete non-final entry, moving the final entry and repairing its index | [x] |
| 34 | `stbds_hmdel_key` | deletions cross shrink threshold after prior table growth | [x] |
| 35 | `stbds_hmdel_key` | deletions cross tombstone rebuild threshold without shrinking | [x] |
| 36 | `stbds_hmdel_key` | string modes default, strdup (free deleted key), and arena | [x] |
| 37 | `stbds_hmfree_func` | null, binary table, string default, string strdup, and string arena | [x] |
| 38 | `stbds_stralloc` | fresh arena; empty, short, and exact/near 512-byte block strings | [x] |
| 39 | `stbds_stralloc` | repeated small strings consume a block and allocate another | [x] |
| 40 | `stbds_stralloc` | string larger than current block size takes dedicated-block branch | [x] |
| 41 | `stbds_stralloc` | block-growth counter advances up to/capped by the 1 MiB maximum | [x] |
| 42 | `stbds_strreset` | empty arena and arena containing ordinary plus dedicated blocks | [x] |
| 43 | `strkey` | negative, zero, positive, `INT_MIN`, and `INT_MAX` values | [x] |
| 44 | `arr_del` | negative, zero, positive, `INT_MIN`, and `INT_MAX`; all four delete and delete-swap indices | [x] |
| 45 | all hash-map entry points | out-of-range modes on both sides of the `mode >= 1` boundary | [x] |
