# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
```

## C-defined public symbols

| symbol | C type | Rust `.so` export | status |
|--------|--------|-------------------|--------|
| `driver` | `T` (global text) | `T driver` | [x] |

The defined-symbol diff is empty:

```text
comm -23 \
  <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

## C undefined runtime symbols

These are dynamic imports rather than symbols implemented by this library.

| symbol | type | classification |
|--------|------|----------------|
| `_ITM_deregisterTMCloneTable` | weak undefined | toolchain runtime |
| `_ITM_registerTMCloneTable` | weak undefined | toolchain runtime |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | libc runtime |
| `__gmon_start__` | weak undefined | toolchain runtime |
| `printf@GLIBC_2.2.5` | undefined | libc |
| `putchar@GLIBC_2.2.5` | undefined | libc |

There are zero missing C-defined symbols and zero undefined non-libc API
symbols that Rust must implement.
