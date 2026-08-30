# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
nm -D target/release/libdriver.so
```

## Library-defined public symbols

| C symbol | C type | Rust type | Present in Rust |
|----------|--------|-----------|-----------------|
| `bad` | `T` | `T` | [x] |
| `driver` | `T` | `T` | [x] |
| `good` | `T` | `T` | [x] |
| `printIntLine` | `T` | `T` | [x] |
| `printLine` | `T` | `T` | [x] |

## Undefined runtime symbols in the C dynamic table

These are imports, not symbols defined or exported by the library. They are
listed to account for every non-empty row emitted by `nm -D` on the C shared
object.

| C dynamic symbol | C type | Rust dynamic table |
|------------------|--------|--------------------|
| `_ITM_deregisterTMCloneTable` | `w` | `w` |
| `_ITM_registerTMCloneTable` | `w` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` |
| `__gmon_start__` | `w` | `w` |
| `printf@GLIBC_2.2.5` | `U` | `U` |
| `puts@GLIBC_2.2.5` | `U` | `U` |

Missing C-defined symbols in Rust: **0**.

