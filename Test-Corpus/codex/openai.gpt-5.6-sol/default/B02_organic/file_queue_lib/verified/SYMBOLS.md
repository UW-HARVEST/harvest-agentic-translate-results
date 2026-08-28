# Dynamic Symbol Surface

Generated from:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

Only globally defined dynamic symbols (`T`) are part of this table. Runtime
and libc imports are not library definitions.

| # | C symbol | C type | Rust symbol | Status |
|---|----------|--------|-------------|--------|
| 1 | `FreeAlertData` | `T` | `FreeAlertData` | [x] |
| 2 | `GetAlertData` | `T` | `GetAlertData` | [x] |
| 3 | `Init_FileQueue` | `T` | `Init_FileQueue` | [x] |
| 4 | `Read_FileMon` | `T` | `Read_FileMon` | [x] |
| 5 | `driver` | `T` | `driver` | [x] |
| 6 | `merror` | `T` | `merror` | [x] |
| 7 | `os_calloc` | `T` | `os_calloc` | [x] |
| 8 | `os_realloc` | `T` | `os_realloc` | [x] |
| 9 | `os_strdup` | `T` | `os_strdup` | [x] |

- [x] Missing C symbols in Rust: **0**.
- [x] Extra Rust exports: **0**.
- [x] Unresolved symbols from `ldd -r target/release/libdriver.so`: **0**.

The Rust library's undefined dynamic entries are satisfied by its declared
runtime dependencies (`libc.so.6`, `libgcc_s.so.1`, and the ELF loader). No
symbol owned by this C library remains undefined.
