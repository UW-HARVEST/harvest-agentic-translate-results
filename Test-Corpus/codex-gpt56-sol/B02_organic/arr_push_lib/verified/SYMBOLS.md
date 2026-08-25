# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libarr_push_lib.so
```

The C and Rust shared libraries each define the same 16 public symbols.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `arr_push` | [x] |
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

Missing from Rust: none.

Undefined non-libc symbols in Rust: none. The remaining undefined symbols are
provided by libc, libgcc unwinding, or the platform runtime.
