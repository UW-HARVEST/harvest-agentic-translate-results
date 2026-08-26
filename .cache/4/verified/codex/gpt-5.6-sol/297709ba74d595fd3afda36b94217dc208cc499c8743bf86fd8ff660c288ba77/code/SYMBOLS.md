# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

The CMake project defines an executable only. The shared object was therefore
linked from the same `c_src/src/lib.c` translation unit with:

```text
/usr/bin/cc -fPIC -shared src/lib.c -o build/libdriver_c.so
```

## Defined Public Symbols

| C address | type | symbol | Rust parity |
|-----------|------|--------|-------------|
| `0000000000001159` | `T` | `process_strings` | [x] |

## C Runtime Imports

These undefined or weak symbols are supplied by the platform C runtime and are
not part of the library API:

```text
_ITM_deregisterTMCloneTable
_ITM_registerTMCloneTable
__cxa_finalize@GLIBC_2.2.5
__gmon_start__
snprintf@GLIBC_2.2.5
strcmp@GLIBC_2.2.5
strlen@GLIBC_2.2.5
strncat@GLIBC_2.2.5
strncmp@GLIBC_2.2.5
strncpy@GLIBC_2.2.5
```

## Missing From Rust

None.
