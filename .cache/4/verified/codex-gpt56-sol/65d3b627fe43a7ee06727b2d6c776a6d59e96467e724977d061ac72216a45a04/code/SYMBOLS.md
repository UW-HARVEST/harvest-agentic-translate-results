# Dynamic Symbol Surface

The default CMake target is an executable. For FFI verification, the unchanged
C source was also compiled as a position-independent shared object:

```sh
cc -shared -fPIC -o c_src/build/libdriver_c.so c_src/src/main.c
```

Derived with:

```sh
nm -D c_src/build/libdriver_c.so
```

## Defined Public Symbols

| symbol | type | C source | Rust parity |
|--------|------|----------|-------------|
| `main` | `T` | `c_src/src/main.c:34` | [x] |

## Undefined Runtime Dependencies

These are imports, not symbols exported by the C library.

| symbol | type |
|--------|------|
| `_ITM_deregisterTMCloneTable` | `w` |
| `_ITM_registerTMCloneTable` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` |
| `__gmon_start__` | `w` |
| `printf@GLIBC_2.2.5` | `U` |
| `puts@GLIBC_2.2.5` | `U` |
| `strlen@GLIBC_2.2.5` | `U` |
| `strtol@GLIBC_2.2.5` | `U` |

## Completion

- [x] `nm -D --defined-only` reports no C symbol missing from the Rust shared
      object.
- [x] The Rust shared object has no undefined non-runtime project symbols.
