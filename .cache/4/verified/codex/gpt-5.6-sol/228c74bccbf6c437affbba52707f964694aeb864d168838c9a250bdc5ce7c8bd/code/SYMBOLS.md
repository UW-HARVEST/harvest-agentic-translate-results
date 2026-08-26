# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libtranslated_rust.so
nm -D target/debug/libdataentry_lib.so
```

## Library-defined API

| C type | symbol | Rust type | status |
|--------|--------|-----------|--------|
| `T` | `dataentry` | `T` | [x] exact export present |

The C header declares only `int dataentry(int, int, int, int)`. All five other
functions in `c_src/src/lib.c` are `static` and are not dynamic symbols.

## Complete C Dynamic Table Classification

| C type | symbol | classification | Rust dynamic table |
|--------|--------|----------------|--------------------|
| `w` | `_ITM_deregisterTMCloneTable` | toolchain weak hook | present |
| `w` | `_ITM_registerTMCloneTable` | toolchain weak hook | present |
| `w` | `__cxa_finalize@GLIBC_2.2.5` | libc/toolchain weak import | present |
| `w` | `__gmon_start__` | toolchain weak hook | present |
| `T` | `dataentry` | library-defined public API | present |
| `U` | `free@GLIBC_2.2.5` | libc import | present |
| `U` | `malloc@GLIBC_2.2.5` | libc import | present |
| `U` | `sprintf@GLIBC_2.2.5` | libc import | not required; Rust does not call it |
| `U` | `strcpy@GLIBC_2.2.5` | libc import | not required; Rust does not call it |
| `U` | `strlen@GLIBC_2.2.5` | libc import | present |

Undefined libc functions are dependencies, not symbols exported by the C
library. There are zero missing library-defined symbols and zero undefined
non-libc API symbols.

