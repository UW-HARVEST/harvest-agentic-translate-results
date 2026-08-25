# Dynamic Symbol Surface

Derived with:

```sh
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `bad` | `T` | `bad` | [x] |
| `driver` | `T` | `driver` | [x] |
| `good` | `T` | `good` | [x] |
| `printIntLine` | `T` | `printIntLine` | [x] |
| `printLine` | `T` | `printLine` | [x] |

The C library's undefined dynamic symbols are libc/toolchain imports:
`printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, and
`__gmon_start__`. It has no undefined project-library symbols.

Missing Rust exports: 0.
