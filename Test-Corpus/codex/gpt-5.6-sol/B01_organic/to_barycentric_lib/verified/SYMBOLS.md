# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `to_barycentric` | function (`T`) | `to_barycentric` | present |

The C library exports one public symbol. The Rust library exports the same
symbol with the exact name. The C library has no undefined project symbols;
its only undefined dynamic symbols are standard toolchain/runtime symbols.

- [x] 0 C symbols missing from the Rust shared library
- [x] 0 undefined non-libc project symbols in the Rust shared library

