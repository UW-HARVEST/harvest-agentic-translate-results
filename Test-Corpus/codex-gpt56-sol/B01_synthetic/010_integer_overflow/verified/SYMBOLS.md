# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`, built from the unchanged
`c_src/src/main.c`.

Command:

```text
nm -D c_src/build/libdriver_c.so
```

| C symbol | C kind | Rust status |
|----------|--------|-------------|
| `_ITM_deregisterTMCloneTable` | weak undefined toolchain hook | not library-owned |
| `_ITM_registerTMCloneTable` | weak undefined toolchain hook | not library-owned |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined libc runtime import | imported by Rust |
| `__gmon_start__` | weak undefined toolchain hook | not library-owned |
| `__isoc99_fscanf@GLIBC_2.7` | undefined libc import | imported by Rust |
| `main` | defined public API | exported |
| `printHexCharLine` | defined public API | exported |
| `printf@GLIBC_2.2.5` | undefined libc import | imported by Rust |
| `stdin@GLIBC_2.2.5` | undefined libc data import | imported by Rust |

Defined C API parity:

| C export | Rust export | Missing |
|----------|-------------|---------|
| `main` | `main` | no |
| `printHexCharLine` | `printHexCharLine` | no |

Completion status: **0 missing defined C symbols; 0 undefined non-libc
library symbols.**
