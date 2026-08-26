# Dynamic Symbol Surface

Source command:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The C shared object exports 16 defined public symbols. The Rust comparison uses
`target/debug/libintput_lib.so`.

| # | symbol | C type | Rust export |
|---|--------|--------|-------------|
| 1 | `intput` | `T` | present |
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

Missing C symbols in Rust: **0**.

The C object's undefined dynamic symbols are libc/toolchain imports:
`__assert_fail`, `free`, `malloc`, `memcmp`, `memcpy`, `memmove`, `memset`,
`realloc`, `sprintf`, `strcmp`, and `strlen`, plus weak ELF runtime hooks.
There are no undefined project symbols.
