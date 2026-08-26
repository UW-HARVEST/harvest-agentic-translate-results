# Dynamic Symbol Surface

Derived with:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libarr_ins_lib.so
```

The C shared library exports 16 public symbols. The Rust shared library exports
all 16 with exact names.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `arr_ins` | [x] |
| 2 | `stbds_arrfreef` | [x] |
| 3 | `stbds_arrgrowf` | [x] |
| 4 | `stbds_hash_bytes` | [x] |
| 5 | `stbds_hash_string` | [x] |
| 6 | `stbds_hmdel_key` | [x] |
| 7 | `stbds_hmfree_func` | [x] |
| 8 | `stbds_hmget_key` | [x] |
| 9 | `stbds_hmget_key_ts` | [x] |
| 10 | `stbds_hmput_default` | [x] |
| 11 | `stbds_hmput_key` | [x] |
| 12 | `stbds_rand_seed` | [x] |
| 13 | `stbds_shmode_func` | [x] |
| 14 | `stbds_stralloc` | [x] |
| 15 | `stbds_strreset` | [x] |
| 16 | `strkey` | [x] |

Missing C symbols in Rust: **0**

Undefined non-runtime/library C symbols in Rust: **0**
