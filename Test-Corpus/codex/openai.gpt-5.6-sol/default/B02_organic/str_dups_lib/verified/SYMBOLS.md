# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-OCFKN1.so
nm -D --defined-only target/release/libstr_dups_lib.so
```

Only defined public symbols are listed. The C library exports 16 symbols.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `stbds_arrfreef` | present |
| 2 | `stbds_arrgrowf` | present |
| 3 | `stbds_hash_bytes` | present |
| 4 | `stbds_hash_string` | present |
| 5 | `stbds_hmdel_key` | present |
| 6 | `stbds_hmfree_func` | present |
| 7 | `stbds_hmget_key` | present |
| 8 | `stbds_hmget_key_ts` | present |
| 9 | `stbds_hmput_default` | present |
| 10 | `stbds_hmput_key` | present |
| 11 | `stbds_rand_seed` | present |
| 12 | `stbds_shmode_func` | present |
| 13 | `stbds_stralloc` | present |
| 14 | `stbds_strreset` | present |
| 15 | `str_dups` | present |
| 16 | `strkey` | present |

Completion check:

- [x] Release libraries rebuilt after all fixes.
- [x] Sorted C-minus-Rust symbol diff is empty.
- [x] Rust has no undefined non-libc project symbols.
