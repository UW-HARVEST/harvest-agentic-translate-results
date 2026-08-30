# Dynamic Symbol Surface

Derived from:

```text
nm -D c_src/build/libdriver.so
nm -D translation/target/release/libdriver.so
```

## Defined public symbols

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `bad` | `T` | `bad` | `T` | present |
| `driver` | `T` | `driver` | `T` | present |
| `good` | `T` | `good` | `T` | present |
| `printLine` | `T` | `printLine` | `T` | present |

Missing defined C symbols: **0**

## Undefined runtime dependencies shown by `nm -D`

These are dynamic imports, not functions defined or exported by this library.
All five also appear in the Rust shared object's dynamic symbol table.

| C dynamic symbol | Type | Rust dynamic table |
|------------------|------|--------------------|
| `_ITM_deregisterTMCloneTable` | `w` | present |
| `_ITM_registerTMCloneTable` | `w` | present |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | present |
| `__gmon_start__` | `w` | present |
| `puts@GLIBC_2.2.5` | `U` | present |

Undefined non-libc application symbols: **0**

