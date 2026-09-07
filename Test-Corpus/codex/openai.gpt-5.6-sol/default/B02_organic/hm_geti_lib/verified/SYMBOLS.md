# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-aKhAwV.so
nm -D --defined-only target/release/libhm_geti_lib.so
```

The C shared object exports 16 defined public symbols. The Rust shared object
currently exports all 16 with identical names.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `hm_geti` | [x] |
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

Undefined C dependencies are libc/toolchain symbols only:
`__assert_fail`, `free`, `memcmp`, `memmove`, `memset`, `realloc`, `sprintf`,
`strcmp`, and `strlen`.
