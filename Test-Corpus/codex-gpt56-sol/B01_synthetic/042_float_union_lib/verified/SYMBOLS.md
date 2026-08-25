# Dynamic Symbol Surface

Source: `nm -D c_src/build/libdriver.so`, built from the unmodified default
CMake configuration.

## Defined public symbols

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `driver` | `T` | `driver` | [x] |

The set difference between C and Rust defined dynamic symbols is empty.

## Undefined runtime symbols

These are toolchain or libc imports, not library API exports:

| symbol |
|--------|
| `_ITM_deregisterTMCloneTable` |
| `_ITM_registerTMCloneTable` |
| `__cxa_finalize@GLIBC_2.2.5` |
| `__gmon_start__` |
| `printf@GLIBC_2.2.5` |
