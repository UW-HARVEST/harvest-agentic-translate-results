# Dynamic Symbol Surface

Generated from `nm -D ../c_src/build/libdriver.so`.

## Defined public API

| symbol | C `.so` | Rust `.so` | status |
|--------|----------|------------|--------|
| `driver` | `T` | `T` | [x] exact export present |

## Undefined and weak runtime imports

These entries are not library API definitions. They are included so every line
reported by `nm -D` on the C shared object is accounted for.

| symbol | C classification | Rust resolution |
|--------|------------------|-----------------|
| `_ITM_deregisterTMCloneTable` | weak undefined toolchain hook | weak undefined toolchain hook |
| `_ITM_registerTMCloneTable` | weak undefined toolchain hook | weak undefined toolchain hook |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined libc import | weak undefined libc import |
| `__gmon_start__` | weak undefined toolchain hook | weak undefined toolchain hook |
| `printf@GLIBC_2.2.5` | undefined libc import | undefined libc import |

Defined-symbol comparison:

```text
C:    driver
Rust: driver
Missing from Rust: (none)
```
