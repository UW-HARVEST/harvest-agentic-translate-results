# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libdriver_c.so
```

## Defined public C symbols

| symbol | C kind | Rust `.so` status |
|--------|--------|-------------------|
| `driver` | `T` | exported as `driver` |
| `main` | `T` | exported as `main` |

## Dynamic runtime dependencies

These are not library entry points. They are weak ELF/runtime hooks or
undefined glibc symbols resolved by the dynamic loader.

| symbol | C kind | classification |
|--------|--------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | weak toolchain hook |
| `_ITM_registerTMCloneTable` | `w` | weak toolchain hook |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | glibc runtime |
| `__gmon_start__` | `w` | weak toolchain hook |
| `__isoc99_scanf@GLIBC_2.7` | `U` | glibc input |
| `printf@GLIBC_2.2.5` | `U` | glibc output |
| `putchar@GLIBC_2.2.5` | `U` | glibc output |

## Parity

- [x] `nm -D --defined-only` reports no C symbol missing from the Rust `.so`.
- [x] There are no undefined non-libc C library symbols.
