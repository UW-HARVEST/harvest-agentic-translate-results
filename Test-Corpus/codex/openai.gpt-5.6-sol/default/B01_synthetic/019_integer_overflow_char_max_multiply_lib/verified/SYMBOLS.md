# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

The C shared library exports these five defined public symbols. The Rust status
was obtained from `nm -D --defined-only target/release/libdriver.so`.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `bad` | `T` | `bad` | present |
| `driver` | `T` | `driver` | present |
| `good` | `T` | `good` | present |
| `printHexCharLine` | `T` | `printHexCharLine` | present |
| `printLine` | `T` | `printLine` | present |

Undefined C dynamic symbols are libc/toolchain dependencies, not library API:
`printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, `__cxa_finalize@GLIBC_2.2.5`,
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, and
`__gmon_start__`.

Missing defined C symbols in Rust: **0**.
