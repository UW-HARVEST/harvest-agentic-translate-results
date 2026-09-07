# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `driver` | `T` | `driver` | `T` | [x] present with exact name |

The complete `nm -D` output also lists weak toolchain symbols and undefined
runtime dependencies. The C library's undefined functional dependencies are
libc `div` and `printf`; both are also undefined runtime dependencies of the
Rust library. There are no undefined project/library symbols.

Defined-symbol diff: empty.

