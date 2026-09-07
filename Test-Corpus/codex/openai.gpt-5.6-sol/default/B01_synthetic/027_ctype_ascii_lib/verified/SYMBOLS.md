# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `driver` | `T` | `driver` | `T` | present |

The complete C defined dynamic-symbol set contains one symbol. The Rust
defined dynamic-symbol set contains the same symbol, with no missing entries.

- [x] Final defined-symbol diff is empty.

The C library's undefined dynamic symbols are libc/toolchain imports:
`__ctype_b_loc`, `printf`, `setlocale`, `tolower`, and `toupper`, plus weak
runtime hooks. They are dependencies rather than symbols implemented by this
library and therefore are not export-parity targets.
