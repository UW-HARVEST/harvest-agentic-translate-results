# Dynamic symbol surface

Generated from:

```text
nm -D c_src/build/libdriver.so
nm -D target/release/libdriver.so
```

| C symbol | C binding | C status | Rust status |
|----------|-----------|----------|-------------|
| `_ITM_deregisterTMCloneTable` | weak | undefined runtime import | present |
| `_ITM_registerTMCloneTable` | weak | undefined runtime import | present |
| `__cxa_finalize@GLIBC_2.2.5` | weak | undefined libc import | present |
| `__gmon_start__` | weak | undefined runtime import | present |
| `driver` | global | defined public API | defined public API |
| `printf@GLIBC_2.2.5` | global | undefined libc import | present |
| `putchar@GLIBC_2.2.5` | global | undefined libc import | present |

`nm -D --defined-only` reports exactly one symbol for each library:
`driver`. The defined-symbol difference is empty, and there are no missing or
undefined non-libc API symbols in Rust.
