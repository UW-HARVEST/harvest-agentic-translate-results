# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-EZOeop.so
nm -D --defined-only target/release/libarr_ins_lib.so
```

Only globally defined dynamic symbols (`T`, `W`, `D`, `B`, or `R`) are part of
this table. Undefined libc/libm imports are not implementation symbols.

| # | C symbol | C type | Rust type | Present in Rust |
|---|----------|--------|-----------|-----------------|
| 1 | `arr_ins` | `T` | `T` | [x] |
| 2 | `stbds_arrfreef` | `T` | `T` | [x] |
| 3 | `stbds_arrgrowf` | `T` | `T` | [x] |
| 4 | `stbds_hash_bytes` | `T` | `T` | [x] |
| 5 | `stbds_hash_string` | `T` | `T` | [x] |
| 6 | `stbds_hmdel_key` | `T` | `T` | [x] |
| 7 | `stbds_hmfree_func` | `T` | `T` | [x] |
| 8 | `stbds_hmget_key` | `T` | `T` | [x] |
| 9 | `stbds_hmget_key_ts` | `T` | `T` | [x] |
| 10 | `stbds_hmput_default` | `T` | `T` | [x] |
| 11 | `stbds_hmput_key` | `T` | `T` | [x] |
| 12 | `stbds_rand_seed` | `T` | `T` | [x] |
| 13 | `stbds_shmode_func` | `T` | `T` | [x] |
| 14 | `stbds_stralloc` | `T` | `T` | [x] |
| 15 | `stbds_strreset` | `T` | `T` | [x] |
| 16 | `strkey` | `T` | `T` | [x] |

## Symbol diff

Missing from Rust: **0**

Extra Rust implementation symbols: **0**

