# Dynamic symbol surface

Source library: `../c_src/build/libdriver.so`

Inventory command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| symbol | nm type | C source | Rust export | status |
|--------|---------|----------|-------------|--------|
| `w_utf8_drop` | `T` | `src/lib.c:39` | `src/lib.rs` | present |
| `w_utf8_filter` | `T` | `src/lib.c:59` | `src/lib.rs` | present |

The C library has no other defined dynamic symbols. Its undefined dynamic
symbols are libc/toolchain imports, not library API. The defined-symbol
difference against `target/release/libdriver.so` is empty.

- [x] Final `nm -D --defined-only` diff: 0 missing and 0 extra symbols.
- [x] Final `ldd -r`: 0 unresolved symbols in the Rust shared library.
