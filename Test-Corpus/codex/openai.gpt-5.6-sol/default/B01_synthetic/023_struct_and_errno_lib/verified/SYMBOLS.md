# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Inventory command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` (`T`) | present |
| `run` | `T` | `run` (`T`) | present |

The C library has no other defined dynamic symbols. Its undefined symbols are
glibc/toolchain imports (`__errno_location`, `printf`, `puts`, `strtol`, and
weak ELF runtime hooks), not library API symbols. The Rust library has zero
missing C API symbols.
