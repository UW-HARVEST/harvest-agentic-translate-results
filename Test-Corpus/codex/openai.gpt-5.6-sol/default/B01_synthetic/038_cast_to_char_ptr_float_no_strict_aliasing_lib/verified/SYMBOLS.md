# Dynamic Symbol Surface

Reference library: `../c_src/build/libdriver.so`

Derived with:

```text
nm -D ../c_src/build/libdriver.so
nm -D --defined-only ../c_src/build/libdriver.so
```

## Public defined symbols

| symbol | C `.so` | Rust `.so` | status |
|--------|----------|------------|--------|
| `driver` | `0000000000001173 T driver` | `00000000000118b0 T driver` | present |

Missing C public symbols in Rust: **0** — [x] verified after the final release
build with an empty `comm -23` diff.

## Undefined imports in the C `.so`

These are dynamic imports, not public functions implemented by this library:

| symbol | kind |
|--------|------|
| `_ITM_deregisterTMCloneTable` | weak toolchain import |
| `_ITM_registerTMCloneTable` | weak toolchain import |
| `__cxa_finalize@GLIBC_2.2.5` | weak libc import |
| `__gmon_start__` | weak toolchain import |
| `printf@GLIBC_2.2.5` | libc import |
| `putchar@GLIBC_2.2.5` | libc import |

The C library has no undefined project/library symbols. `ldd -r` reports no
unresolved relocations for either shared library.
