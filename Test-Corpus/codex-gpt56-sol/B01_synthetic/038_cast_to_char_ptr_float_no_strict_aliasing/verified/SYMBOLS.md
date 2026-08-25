# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`

Command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

| symbol | C type | Rust parity |
|--------|--------|-------------|
| `driver` | `T` | [x] |
| `main` | `T` | [x] |

The complete `nm -D` output also contains the undefined libc imports
`__isoc99_scanf`, `printf`, and `putchar`, plus weak ELF/toolchain hooks
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
and `__gmon_start__`. These are dynamic dependencies, not symbols defined or
exported by the C library.

Completion gate: [x] no C-defined dynamic symbol is missing from the Rust
shared object.
