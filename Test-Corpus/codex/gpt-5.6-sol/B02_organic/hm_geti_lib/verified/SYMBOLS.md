# Dynamic Symbol Surface

Derived with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The C build exports 16 public symbols. `Rust export` records the same-name
symbol in `target/debug/libhm_geti_lib.so`.

| # | C symbol | Kind | Rust export |
|---|----------|------|-------------|
| 1 | `hm_geti` | `T` | [x] |
| 2 | `stbds_arrfreef` | `T` | [x] |
| 3 | `stbds_arrgrowf` | `T` | [x] |
| 4 | `stbds_hash_bytes` | `T` | [x] |
| 5 | `stbds_hash_string` | `T` | [x] |
| 6 | `stbds_hmdel_key` | `T` | [x] |
| 7 | `stbds_hmfree_func` | `T` | [x] |
| 8 | `stbds_hmget_key` | `T` | [x] |
| 9 | `stbds_hmget_key_ts` | `T` | [x] |
| 10 | `stbds_hmput_default` | `T` | [x] |
| 11 | `stbds_hmput_key` | `T` | [x] |
| 12 | `stbds_rand_seed` | `T` | [x] |
| 13 | `stbds_shmode_func` | `T` | [x] |
| 14 | `stbds_stralloc` | `T` | [x] |
| 15 | `stbds_strreset` | `T` | [x] |
| 16 | `strkey` | `T` | [x] |

Missing from Rust: **0**.

