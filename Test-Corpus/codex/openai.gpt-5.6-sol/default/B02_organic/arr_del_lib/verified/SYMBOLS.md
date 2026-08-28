# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --format=posix ../c_src/build/libharvest-work-wMaLju.so
nm -D --defined-only --format=posix target/release/libarr_del_lib.so
```

| # | C symbol | Rust export | Status |
|---|----------|-------------|--------|
| 1 | `arr_del` | `arr_del` | [x] |
| 2 | `stbds_arrfreef` | `stbds_arrfreef` | [x] |
| 3 | `stbds_arrgrowf` | `stbds_arrgrowf` | [x] |
| 4 | `stbds_hash_bytes` | `stbds_hash_bytes` | [x] |
| 5 | `stbds_hash_string` | `stbds_hash_string` | [x] |
| 6 | `stbds_hmdel_key` | `stbds_hmdel_key` | [x] |
| 7 | `stbds_hmfree_func` | `stbds_hmfree_func` | [x] |
| 8 | `stbds_hmget_key` | `stbds_hmget_key` | [x] |
| 9 | `stbds_hmget_key_ts` | `stbds_hmget_key_ts` | [x] |
| 10 | `stbds_hmput_default` | `stbds_hmput_default` | [x] |
| 11 | `stbds_hmput_key` | `stbds_hmput_key` | [x] |
| 12 | `stbds_rand_seed` | `stbds_rand_seed` | [x] |
| 13 | `stbds_shmode_func` | `stbds_shmode_func` | [x] |
| 14 | `stbds_stralloc` | `stbds_stralloc` | [x] |
| 15 | `stbds_strreset` | `stbds_strreset` | [x] |
| 16 | `strkey` | `strkey` | [x] |

Missing C symbols in Rust: **0**

Undefined non-runtime C symbols in Rust: **0**
