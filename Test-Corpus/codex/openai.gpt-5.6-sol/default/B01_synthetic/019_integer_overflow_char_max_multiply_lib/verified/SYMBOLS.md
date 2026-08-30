# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust `.so` status |
|----------|--------|-------------------|
| `bad` | `T` | present as `T` |
| `driver` | `T` | present as `T` |
| `good` | `T` | present as `T` |
| `printHexCharLine` | `T` | present as `T` |
| `printLine` | `T` | present as `T` |

The C library's undefined dynamic symbols are `printf` and `puts` from libc,
plus weak ELF/toolchain symbols. It has no undefined non-libc library symbols.

Completion status: [x] verified after the final release build
