# Dynamic symbol surface

Mechanically derived with:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | ELF type | Rust export | Status |
|----------|----------|-------------|--------|
| `driver` | `T` | `driver` | present |
| `printLine` | `T` | `printLine` | present |

Missing C symbols in Rust: **0**.

The C library's remaining undefined dynamic symbols are libc/toolchain symbols
(`memset`, `puts`, `strncpy`, and weak ELF runtime hooks), not library API
symbols.
