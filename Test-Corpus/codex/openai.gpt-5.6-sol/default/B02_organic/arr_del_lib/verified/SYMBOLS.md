# Exported symbol surface

Source of truth:

```text
nm -D --defined-only ../c_src/build/libharvest-work-Wqkjij.so
```

The C shared object exports 16 project symbols. The Rust release shared object
exports all 16 with the exact same names.

| # | C symbol | Rust export | status |
|---|----------|-------------|--------|
| 1 | `arr_del` | `arr_del` | present |
| 2 | `stbds_arrfreef` | `stbds_arrfreef` | present |
| 3 | `stbds_arrgrowf` | `stbds_arrgrowf` | present |
| 4 | `stbds_hash_bytes` | `stbds_hash_bytes` | present |
| 5 | `stbds_hash_string` | `stbds_hash_string` | present |
| 6 | `stbds_hmdel_key` | `stbds_hmdel_key` | present |
| 7 | `stbds_hmfree_func` | `stbds_hmfree_func` | present |
| 8 | `stbds_hmget_key` | `stbds_hmget_key` | present |
| 9 | `stbds_hmget_key_ts` | `stbds_hmget_key_ts` | present |
| 10 | `stbds_hmput_default` | `stbds_hmput_default` | present |
| 11 | `stbds_hmput_key` | `stbds_hmput_key` | present |
| 12 | `stbds_rand_seed` | `stbds_rand_seed` | present |
| 13 | `stbds_shmode_func` | `stbds_shmode_func` | present |
| 14 | `stbds_stralloc` | `stbds_stralloc` | present |
| 15 | `stbds_strreset` | `stbds_strreset` | present |
| 16 | `strkey` | `strkey` | present |

Missing C symbols in Rust: **0**.

Undefined non-libc project symbols in Rust: **0**.

