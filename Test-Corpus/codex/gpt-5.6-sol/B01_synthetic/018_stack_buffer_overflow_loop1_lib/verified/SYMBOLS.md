# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

## C-defined public symbols

| symbol | C type | Rust type | Rust export present |
|--------|--------|-----------|---------------------|
| `bad` | `T` | `T` | [x] |
| `driver` | `T` | `T` | [x] |
| `good` | `T` | `T` | [x] |
| `printIntLine` | `T` | `T` | [x] |
| `printLine` | `T` | `T` | [x] |

Missing C-defined symbols in Rust: **0**

## C external dynamic symbols

These are C toolchain/libc dependencies reported by the unfiltered `nm -D`
output, not symbols defined by the driver library.

| symbol | C type | provider |
|--------|--------|----------|
| `_ITM_deregisterTMCloneTable` | `w` | compiler runtime, optional weak import |
| `_ITM_registerTMCloneTable` | `w` | compiler runtime, optional weak import |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc, optional weak import |
| `__gmon_start__` | `w` | compiler profiling runtime, optional weak import |
| `printf@GLIBC_2.2.5` | `U` | libc |
| `puts@GLIBC_2.2.5` | `U` | libc |

Undefined non-libc application symbols: **0**
