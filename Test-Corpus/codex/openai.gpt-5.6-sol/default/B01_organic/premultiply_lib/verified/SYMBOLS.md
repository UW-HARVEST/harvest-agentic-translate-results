# Dynamic symbol surface

Generated from:

```text
nm -D ../c_src/build/libharvest-work-9Toehl.so
nm -D --defined-only target/release/libpremultiply_lib.so
```

## C dynamic symbols

| Type | Symbol | Classification |
|---|---|---|
| `w` | `_ITM_deregisterTMCloneTable` | Toolchain weak undefined symbol |
| `w` | `_ITM_registerTMCloneTable` | Toolchain weak undefined symbol |
| `w` | `__cxa_finalize@GLIBC_2.2.5` | Toolchain/libc weak undefined symbol |
| `w` | `__gmon_start__` | Toolchain weak undefined symbol |
| `T` | `premultiply` | Public library export |

## Required defined-export parity

| C symbol | Rust symbol | Status |
|---|---|---|
| `premultiply` | `premultiply` | [x] exact-name export present |

There are no missing C-defined public symbols in the Rust shared library.
The four lowercase-`w` entries are undefined toolchain hooks, not library
implementations.

