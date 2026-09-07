# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust `.so` export | Status |
|----------|--------|-------------------|--------|
| `driver` | `T` | `driver` | [x] present |
| `printHexCharLine` | `T` | `printHexCharLine` | [x] present |

The C library's only undefined dynamic symbol is the libc function
`printf@GLIBC_2.2.5`. There are no undefined non-libc project symbols.

Final comparison: zero C-defined dynamic symbols are missing from the Rust
shared object.
