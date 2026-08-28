# Configuration Surface

The public ABI is the complete 16-symbol `nm -D` surface in `SYMBOLS.md`.
`STBDS_HM_BINARY` is 0, `STBDS_HM_STRING` is 1; string storage modes are
none/default/strdup/arena (0/1/2/3). Rows are branch-equivalence classes
derived from the C implementation.

| # | entry point(s) | configuration (options set + input shape) | Test |
|---|----------------|--------------------------------------------|------|
| 1 | `stbds_arrgrowf` | Null array; positive requested length/capacity below 4, selecting initial capacity 4; randomized element sizes and contents | [x] |
| 2 | `stbds_arrgrowf` | Existing array; request fits capacity and returns unchanged | [x] |
| 3 | `stbds_arrgrowf` | Existing array; `min_len > min_cap` and growth doubles old capacity | [x] |
| 4 | `stbds_arrgrowf` | Null/existing array; explicit `min_cap` dominates length and old doubled capacity | [x] |
| 5 | `stbds_arrfreef` | Free each non-null array shape produced by rows 1-4 | [x] |
| 6 | `stbds_rand_seed`, `stbds_hash_bytes` | Empty input (`len == 0`) with zero/nonzero seeds | [x] |
| 7 | `stbds_hash_bytes` | Tail lengths 1 through 7, exercising every fall-through case, randomized bytes/seeds | [x] |
| 8 | `stbds_hash_bytes` | One and many full 8-byte words, with and without a 1-7 byte tail | [x] |
| 9 | `stbds_hash_string` | Empty, short, long, and high-bit-byte NUL-terminated strings with randomized seeds | [x] |
| 10 | `stbds_hmput_default`, `stbds_hmget_key`, `stbds_hmget_key_ts` | Null map becomes a zeroed default record; repeated default call is unchanged | [x] |
| 11 | `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts` | Binary mode, randomized key widths/element sizes; insert new then update existing key | [x] |
| 12 | `stbds_hmput_key`, `stbds_hmget_key`, `stbds_rand_seed` | Binary mode; enough randomized keys for 8-slot initialization and repeated table growth/rehash | [x] |
| 13 | `stbds_hmdel_key` | Binary mode; delete last element versus non-last element (move and repair index) | [x] |
| 14 | `stbds_hmdel_key`, `stbds_hmput_key` | Binary mode; deletions create/reuse tombstones and exceed rebuild threshold | [x] |
| 15 | `stbds_hmdel_key` | Binary mode; enough insertions/deletions to cross shrink threshold | [x] |
| 16 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts` | String mode with default pointer storage; empty/short/long randomized keys, insert/update/get | [x] |
| 17 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key` | String mode with strdup storage; mutate source after insertion and verify copied key | [x] |
| 18 | `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key` | String mode with arena storage; many small keys plus keys over current block size | [x] |
| 19 | `stbds_hmdel_key` | String default/strdup/arena modes; missing, last, and non-last deletion | [x] |
| 20 | `stbds_hmfree_func` | Binary and all three string-storage table modes, empty and populated | [x] |
| 21 | `stbds_stralloc` | Empty/small strings fitting a fresh 512-byte block and then fitting remaining space | [x] |
| 22 | `stbds_stralloc` | String one byte larger than current block size, both with empty and existing arena storage | [x] |
| 23 | `stbds_stralloc` | Repeated allocations grow block size through the 1,048,576-byte cap boundary | [x] |
| 24 | `stbds_strreset` | Empty arena and arena containing ordinary plus oversized blocks | [x] |
| 25 | `stbds_shmode_func` | Every declared mode 0/1/2/3 plus out-of-range integer modes; verify stored low byte and empty-map shape | [x] |
| 26 | `stbds_hmput_key`, `stbds_hmget_key_ts`, `stbds_hmdel_key` | Out-of-range `mode < 1` follows binary branches; `mode > 1` follows string comparison/hash branches | [x] |
| 27 | `stbds_hmdel_key` | Nonzero randomized `keyoffset` for binary and string records | [x] |
| 28 | `strkey` | Negative, zero, positive, `INT_MIN`, and `INT_MAX`; returned static-buffer bytes | [x] |
| 29 | `arr_del` | Negative, zero, positive, `INT_MIN`, and `INT_MAX`; all four ordered-delete and swap-delete indices complete normally | [x] |

## Feature Matrix

`Cargo.toml` declares no named features. The full valid/error matrix passes in
both distinct Cargo modes:

- [x] Default features
- [x] `--no-default-features`
