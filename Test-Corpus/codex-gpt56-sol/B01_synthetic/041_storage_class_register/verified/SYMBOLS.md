# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`, linked from CMake's
position-independent `main.c.o`.

## C-defined public symbols

| symbol | C type | initial Rust status | resolution | final parity |
|---|---|---|---|---|
| `driver` | `T` | missing (binary-only translation) | added the real translated `extern "C"` export | [x] |
| `main` | `T` | missing (binary-only translation) | added the real translated `extern "C"` export | [x] |

## Full `nm -D` dependency surface

The remaining C dynamic-symbol entries are runtime dependencies, not symbols
defined by this library:

| symbol | type |
|---|---|
| `_ITM_deregisterTMCloneTable` | weak undefined |
| `_ITM_registerTMCloneTable` | weak undefined |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined |
| `__gmon_start__` | weak undefined |
| `__isoc99_scanf@GLIBC_2.7` | undefined libc |
| `printf@GLIBC_2.2.5` | undefined libc |

Final missing C-defined symbols: **0**.
