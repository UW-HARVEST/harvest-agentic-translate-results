# Dynamic Symbol Surface

Reference object: `c_src/build/libnineality.so`

Command:

```text
nm -D c_src/build/libnineality.so
```

## C-owned exports

| symbol | nm type | C definition | Rust status |
|--------|---------|--------------|-------------|
| `main` | `T` | `c_src/src/main.c:31` | [x] exported as `main` |

## Dynamic imports and weak runtime hooks

These entries are present in the complete C `nm -D` output but are supplied by
libc or the ELF toolchain, not defined by this library.

| symbol as reported by `nm -D` | type | Rust object contains symbol |
|--------------------------------|------|-----------------------------|
| `_ITM_deregisterTMCloneTable` | `w` | [x] |
| `_ITM_registerTMCloneTable` | `w` | [x] |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | [x] |
| `__gmon_start__` | `w` | [x] |
| `printf@GLIBC_2.2.5` | `U` | [x] |
| `puts@GLIBC_2.2.5` | `U` | [x] |
| `strtol@GLIBC_2.2.5` | `U` | [x] |

## Parity result

- [x] `nm -D --defined-only` reports no C-owned symbol missing from Rust.
- [x] The C object has no undefined non-libc implementation symbol.

