# Dynamic symbol surface

Generated from:

```text
nm -D ../c_src/build/libharvest-work-7UuCY5.so
nm -D target/release/libdataentry_lib.so
```

## Library-defined public symbols

| C address | C type | symbol | Rust address | Rust type | parity |
|-----------|--------|--------|--------------|-----------|--------|
| `00000000000013fb` | `T` | `dataentry` | `0000000000011d40` | `T` | [x] |

Missing C-defined public symbols in Rust: **0**.

## C shared-library runtime imports

These are undefined runtime/toolchain imports shown by `nm -D`, not public
functions implemented by this library:

| type | symbol |
|------|--------|
| `w` | `_ITM_deregisterTMCloneTable` |
| `w` | `_ITM_registerTMCloneTable` |
| `w` | `__cxa_finalize@GLIBC_2.2.5` |
| `w` | `__gmon_start__` |
| `U` | `free@GLIBC_2.2.5` |
| `U` | `malloc@GLIBC_2.2.5` |
| `U` | `sprintf@GLIBC_2.2.5` |
| `U` | `strcpy@GLIBC_2.2.5` |
| `U` | `strlen@GLIBC_2.2.5` |

