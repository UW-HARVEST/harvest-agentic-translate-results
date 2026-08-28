# Dynamic Symbol Surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-xQR7Ht.so
nm -D --defined-only target/release/libarr_ins_lib.so
```

| # | C symbol | ELF type | Rust export |
|---|----------|----------|-------------|
| 1 | `arr_ins` | `T` | present |
| 2 | `stbds_arrfreef` | `T` | present |
| 3 | `stbds_arrgrowf` | `T` | present |
| 4 | `stbds_hash_bytes` | `T` | present |
| 5 | `stbds_hash_string` | `T` | present |
| 6 | `stbds_hmdel_key` | `T` | present |
| 7 | `stbds_hmfree_func` | `T` | present |
| 8 | `stbds_hmget_key` | `T` | present |
| 9 | `stbds_hmget_key_ts` | `T` | present |
| 10 | `stbds_hmput_default` | `T` | present |
| 11 | `stbds_hmput_key` | `T` | present |
| 12 | `stbds_rand_seed` | `T` | present |
| 13 | `stbds_shmode_func` | `T` | present |
| 14 | `stbds_stralloc` | `T` | present |
| 15 | `stbds_strreset` | `T` | present |
| 16 | `strkey` | `T` | present |

Missing C-defined symbols in Rust: **0**.
