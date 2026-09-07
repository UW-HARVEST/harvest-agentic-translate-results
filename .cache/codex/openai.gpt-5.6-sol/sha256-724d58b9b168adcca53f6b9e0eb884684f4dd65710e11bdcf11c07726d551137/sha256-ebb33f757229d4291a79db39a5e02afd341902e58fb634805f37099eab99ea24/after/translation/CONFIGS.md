# Configuration surface

Rows are derived from the branches in `src/lib.c`: null/existing storage,
capacity relations, SipHash tail widths, binary versus string comparison,
string ownership mode, new/update/missing key, grow/shrink/rebuild thresholds,
arena block sizing, and every exported entry point.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | null array; requested capacity 1..3, forcing minimum capacity 4 | [x] |
| 2 | `stbds_arrgrowf` | null array; explicit capacity at least 4 | [x] |
| 3 | `stbds_arrgrowf` | existing array; requested minimum no greater than capacity (pointer-preserving no-op) | [x] |
| 4 | `stbds_arrgrowf` | existing array; `length + addlen` exceeds capacity and doubling wins | [x] |
| 5 | `stbds_arrgrowf` | existing array; explicit `min_cap` exceeds twice capacity | [x] |
| 6 | `stbds_arrfreef` | free a non-null allocation returned by `stbds_arrgrowf` | [x] |
| 7 | `stbds_rand_seed` | seeds `0`, small values, and full-width values before new table creation | [x] |
| 8 | `stbds_hash_string` | empty string across varied seeds | [x] |
| 9 | `stbds_hash_string` | nonempty ASCII and high-bit bytes across varied lengths/seeds | [x] |
| 10 | `stbds_hash_bytes` | lengths 0 through 7 (every SipHash tail switch case) | [x] |
| 11 | `stbds_hash_bytes` | length exactly 8 (one full word, zero-byte tail) | [x] |
| 12 | `stbds_hash_bytes` | lengths greater than 8 with tails 0 through 7 | [x] |
| 13 | `stbds_hmget_key_ts` | null binary map lookup creates default element and returns index `-1` | [x] |
| 14 | `stbds_hmget_key` | existing default-only binary map with no hash table; missing lookup | [x] |
| 15 | `stbds_hmput_default` | null map creates one zeroed default element | [x] |
| 16 | `stbds_hmput_default` | existing map/default element; pointer-preserving no-op | [x] |
| 17 | `stbds_hmput_key` / `stbds_hmget_key[_ts]` | binary mode, new key, key sizes 1/2/4/8/odd, varied element sizes | [x] |
| 18 | `stbds_hmput_key` / `stbds_hmget_key[_ts]` | binary mode, existing key update path | [x] |
| 19 | `stbds_hmput_key` | binary mode, enough distinct keys to grow table and backing array repeatedly | [x] |
| 20 | `stbds_hmdel_key` | binary mode, delete null map / map without table / absent key | [x] |
| 21 | `stbds_hmdel_key` | binary mode, delete present last element | [x] |
| 22 | `stbds_hmdel_key` | binary mode, delete non-last element and repair moved index | [x] |
| 23 | `stbds_hmdel_key` | binary mode, deletions trigger tombstone rebuild | [x] |
| 24 | `stbds_hmdel_key` | binary mode, deletions trigger table shrink | [x] |
| 25 | `stbds_shmode_func` / map APIs | string mode `STBDS_SH_DEFAULT` (borrowed key), empty/one/many keys | [x] |
| 26 | `stbds_shmode_func` / map APIs | string mode `STBDS_SH_STRDUP` (owned duplicate), update/get/delete/free | [x] |
| 27 | `stbds_shmode_func` / map APIs | string mode `STBDS_SH_ARENA`, update/get/delete/free | [x] |
| 28 | `stbds_hmput_key` / map APIs | direct string mode on null map (implicit `STBDS_SH_DEFAULT`) | [x] |
| 29 | map APIs | out-of-range mode below string threshold (`mode < 1`) follows binary behavior | [x] |
| 30 | map APIs | out-of-range mode above string threshold (`mode > 1`) follows string comparison behavior | [x] |
| 31 | `stbds_shmode_func` | out-of-range storage mode truncates to `unsigned char`; unrecognized switch value uses binary-copy insertion | [x] |
| 32 | `stbds_stralloc` | zeroed arena; short strings share 512-byte initial block | [x] |
| 33 | `stbds_stralloc` | repeated allocations advance arena block sizes up to the 1 MiB cap | [x] |
| 34 | `stbds_stralloc` | string larger than selected block gets a dedicated block, with and without existing storage | [x] |
| 35 | `stbds_strreset` | reset empty and populated arenas; all fields become zero | [x] |
| 36 | `stbds_hmfree_func` | null, binary, borrowed-string, duplicated-string, and arena-string maps | [x] |
| 37 | `strkey` | negative, zero, and positive `int` values; returned static buffer bytes | [x] |
| 38 | `intput` | ordinary values distinct from reserved inserted keys 9 and 11 | [x] |
| 39 | `intput` | colliding values 9 and 11 reach the C assertion rejection | [x] |
| 40 | `stbds_arrgrowf` | null array with `addlen == 0` and `min_cap == 0`; pointer-preserving null no-op | [x] |

Feature combinations from `Cargo.toml`: one (no `[features]` table; default
build only). The crate has no binary target.
