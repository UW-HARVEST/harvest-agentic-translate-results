# Dynamic Symbol Surface

Source: `nm -D c_src/build/libdriver_c.so`, where the shared object is built
from the unchanged `c_src/src/main.c` using `cc -shared -fPIC`.

The supplied CMake file defines an executable only. Its required default build
was run first; the separate shared-object build is necessary for FFI loading.

## Defined public symbols

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `bad` | `T` | `bad` | present |
| `good` | `T` | `good` | present |
| `main` | `T` | `main` | present |
| `printHexCharLine` | `T` | `printHexCharLine` | present |
| `printLine` | `T` | `printLine` | present |

Missing from Rust: **0**

## C external dynamic imports

These are runtime imports rather than API definitions:

| Symbol | Type |
|--------|------|
| `_ITM_deregisterTMCloneTable` | weak |
| `_ITM_registerTMCloneTable` | weak |
| `__cxa_finalize@GLIBC_2.2.5` | weak |
| `__gmon_start__` | weak |
| `__isoc99_scanf@GLIBC_2.7` | undefined libc |
| `printf@GLIBC_2.2.5` | undefined libc |
| `puts@GLIBC_2.2.5` | undefined libc |

There are no undefined non-runtime C symbols.
