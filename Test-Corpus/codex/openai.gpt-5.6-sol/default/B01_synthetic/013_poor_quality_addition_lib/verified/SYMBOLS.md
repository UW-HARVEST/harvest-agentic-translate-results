# Dynamic Symbol Surface

Derived mechanically with:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | C type | Rust symbol present |
|----------|--------|---------------------|
| `bad` | `T` | yes |
| `driver` | `T` | yes |
| `good` | `T` | yes |
| `printIntLine` | `T` | yes |
| `printLine` | `T` | yes |

The C library also has undefined dynamic references to `printf` and `puts`,
plus weak ELF toolchain symbols. These are runtime dependencies rather than
symbols defined by the library.

Missing C-defined symbols in Rust: **0**
