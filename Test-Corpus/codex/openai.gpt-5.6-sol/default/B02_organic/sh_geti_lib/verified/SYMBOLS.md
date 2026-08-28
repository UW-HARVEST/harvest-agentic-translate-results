# Dynamic Symbol Surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-punw4N.so
nm -D --defined-only target/release/libsh_geti_lib.so
```

Only defined public symbols are listed. Undefined GLIBC/toolchain symbols are
external dependencies, not library API.

| # | C symbol | C definition | Rust export | Status |
|---|----------|--------------|-------------|--------|
| 1 | `sh_geti` | `src/lib.c:945` | `src/lib.rs` | [x] |
| 2 | `stbds_arrfreef` | `src/lib.c:312` | `src/lib.rs` | [x] |
| 3 | `stbds_arrgrowf` | `src/lib.c:276` | `src/lib.rs` | [x] |
| 4 | `stbds_hash_bytes` | `src/lib.c:553` | `src/lib.rs` | [x] |
| 5 | `stbds_hash_string` | `src/lib.c:477` | `src/lib.rs` | [x] |
| 6 | `stbds_hmdel_key` | `src/lib.c:807` | `src/lib.rs` | [x] |
| 7 | `stbds_hmfree_func` | `src/lib.c:571` | `src/lib.rs` | [x] |
| 8 | `stbds_hmget_key` | `src/lib.c:659` | `src/lib.rs` | [x] |
| 9 | `stbds_hmget_key_ts` | `src/lib.c:631` | `src/lib.rs` | [x] |
| 10 | `stbds_hmput_default` | `src/lib.c:667` | `src/lib.rs` | [x] |
| 11 | `stbds_hmput_key` | `src/lib.c:680` | `src/lib.rs` | [x] |
| 12 | `stbds_rand_seed` | `src/lib.c:355` | `src/lib.rs` | [x] |
| 13 | `stbds_shmode_func` | `src/lib.c:796` | `src/lib.rs` | [x] |
| 14 | `stbds_stralloc` | `src/lib.c:881` | `src/lib.rs` | [x] |
| 15 | `stbds_strreset` | `src/lib.c:920` | `src/lib.rs` | [x] |
| 16 | `strkey` | `src/lib.c:939` | `src/lib.rs` | [x] |

## Completion

- [x] All 16 C-defined dynamic symbols are defined by the Rust shared object.
- [x] No C library symbol is an undefined non-libc dependency of the Rust shared object.

