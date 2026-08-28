# Dynamic symbol surface

Source library:
`../c_src/build/libharvest-work-gSZO9L.so`

The table is the complete set produced by:

```text
nm -D --defined-only ../c_src/build/libharvest-work-gSZO9L.so
```

| # | C symbol | Rust export | Status |
|---|----------|-------------|--------|
| 1 | `stbds_arrgrowf` | `stbds_arrgrowf` | present |
| 2 | `stbds_arrfreef` | `stbds_arrfreef` | present |
| 3 | `stbds_rand_seed` | `stbds_rand_seed` | present |
| 4 | `stbds_hash_string` | `stbds_hash_string` | present |
| 5 | `stbds_hash_bytes` | `stbds_hash_bytes` | present |
| 6 | `stbds_hmfree_func` | `stbds_hmfree_func` | present |
| 7 | `stbds_hmget_key_ts` | `stbds_hmget_key_ts` | present |
| 8 | `stbds_hmget_key` | `stbds_hmget_key` | present |
| 9 | `stbds_hmput_default` | `stbds_hmput_default` | present |
| 10 | `stbds_hmput_key` | `stbds_hmput_key` | present |
| 11 | `stbds_shmode_func` | `stbds_shmode_func` | present |
| 12 | `stbds_hmdel_key` | `stbds_hmdel_key` | present |
| 13 | `stbds_stralloc` | `stbds_stralloc` | present |
| 14 | `stbds_strreset` | `stbds_strreset` | present |
| 15 | `strkey` | `strkey` | present |
| 16 | `str_put` | `str_put` | present |

Missing symbols: **0**

Cargo features: none declared. The applicable build configurations are the
default invocation and `--no-default-features`.

## Completion

- [x] Default release build: all 16 C symbols are exported by Rust.
- [x] `--no-default-features` release build: all 16 C symbols are exported by Rust.
- [x] Missing C symbols: 0.
