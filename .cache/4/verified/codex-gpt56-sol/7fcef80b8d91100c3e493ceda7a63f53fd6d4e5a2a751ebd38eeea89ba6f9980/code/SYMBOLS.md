# Dynamic Symbol Surface

Derived from:

```text
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001173 T driver
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `driver` | Global function (`T`) | `driver` | Present |

The C library's undefined dynamic references are `printf@GLIBC_2.2.5` and
`putchar@GLIBC_2.2.5`; both are libc symbols, not library API symbols. Its
remaining undefined entries are weak toolchain/runtime symbols.

The defined-symbol comparison is empty:

```text
comm -23 C_DEFINED_SYMBOLS RUST_DEFINED_SYMBOLS
```

- [x] No C API symbol is missing from the Rust shared library.
- [x] No undefined non-libc C library dependency must be supplied by Rust.
