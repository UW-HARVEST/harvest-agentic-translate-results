# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libhelxo_lib.so
```

The C shared object exports 16 defined public symbols. The Rust shared object
exports all 16 with exact names.

| # | C symbol | Rust export | Source |
|---|----------|-------------|--------|
| 1 | `helxo` | present | `c_src/src/lib.c:945` |
| 2 | `stbds_arrfreef` | present | `c_src/src/lib.c:312` |
| 3 | `stbds_arrgrowf` | present | `c_src/src/lib.c:276` |
| 4 | `stbds_hash_bytes` | present | `c_src/src/lib.c:553` |
| 5 | `stbds_hash_string` | present | `c_src/src/lib.c:477` |
| 6 | `stbds_hmdel_key` | present | `c_src/src/lib.c:807` |
| 7 | `stbds_hmfree_func` | present | `c_src/src/lib.c:571` |
| 8 | `stbds_hmget_key` | present | `c_src/src/lib.c:659` |
| 9 | `stbds_hmget_key_ts` | present | `c_src/src/lib.c:631` |
| 10 | `stbds_hmput_default` | present | `c_src/src/lib.c:667` |
| 11 | `stbds_hmput_key` | present | `c_src/src/lib.c:680` |
| 12 | `stbds_rand_seed` | present | `c_src/src/lib.c:355` |
| 13 | `stbds_shmode_func` | present | `c_src/src/lib.c:796` |
| 14 | `stbds_stralloc` | present | `c_src/src/lib.c:881` |
| 15 | `stbds_strreset` | present | `c_src/src/lib.c:920` |
| 16 | `strkey` | present | `c_src/src/lib.c:939` |

Missing C symbols in Rust: **0**.

Undefined C dependencies are only libc/toolchain symbols (`assert`, allocation,
memory, string, and stdio routines plus standard ELF weak hooks). There are no
undefined project symbols in either shared object.
