# Dynamic Symbol Surface

Generated from the default C shared library with:

```sh
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `driver` | `T` | `driver` | present |
| `printLine` | `T` | `printLine` | present |

The exact-name comparison is:

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

It produces no output. The C library's remaining dynamic symbols are undefined
runtime dependencies (`memset`, `puts`, `strncpy`, and weak ELF runtime
symbols), not symbols defined and exported by the library.

- [x] Exact-name symbol diff is empty.
- [x] Rust has no missing C-defined dynamic symbols.
