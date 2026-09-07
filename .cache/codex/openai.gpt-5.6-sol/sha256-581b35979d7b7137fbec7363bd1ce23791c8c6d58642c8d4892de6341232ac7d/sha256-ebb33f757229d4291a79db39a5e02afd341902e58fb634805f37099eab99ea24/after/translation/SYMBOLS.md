# Dynamic symbol surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | C type | Rust export present |
|----------|--------|---------------------|
| `merror` | `T` | [x] |
| `Init_FileQueue` | `T` | [x] |
| `Read_FileMon` | `T` | [x] |
| `os_calloc` | `T` | [x] |
| `os_realloc` | `T` | [x] |
| `os_strdup` | `T` | [x] |
| `FreeAlertData` | `T` | [x] |
| `GetAlertData` | `T` | [x] |
| `driver` | `T` | [x] |

Missing C symbols in Rust: **0**.

The C library's remaining undefined symbols are libc/toolchain symbols. There
are no undefined project-local symbols.
