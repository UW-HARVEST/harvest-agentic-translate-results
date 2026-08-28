# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-lca2wP.so
nm -D --defined-only target/release/libintput_lib.so
```

| # | C symbol | Rust export | Status |
|---|----------|-------------|--------|
| 1 | `intput` | `intput` | present |
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

The undefined symbols in both shared objects are runtime/libc/compiler support
symbols. There are no undefined symbols owned by this library.

## Completion Gate

- [x] `nm -D` reports zero C symbols missing from Rust and zero extra Rust symbols.
- [x] Every `CONFIGS.md` row passes randomized differential testing.
- [x] Every `ERRORS.md` row is covered by sentinel, invariant, or isolated abort testing.
- [x] Default and `--no-default-features` builds/checks/tests pass; no optional features are declared.
