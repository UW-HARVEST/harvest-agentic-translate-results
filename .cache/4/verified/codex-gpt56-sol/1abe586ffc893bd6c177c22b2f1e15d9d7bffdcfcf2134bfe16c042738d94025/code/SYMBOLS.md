# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`, built from the unchanged
`c_src/src/main.c` with `cc -shared -fPIC`. The prescribed CMake target is an
executable and therefore does not produce a shared object.

## Defined API symbols

Mechanically derived with:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

| symbol | C type | Rust export | status |
|---|---|---|---|
| `bad` | `T` | `bad` | [x] |
| `good` | `T` | `good` | [x] |
| `main` | `T` | `main` | [x] |
| `printIntLine` | `T` | `printIntLine` | [x] |
| `printLine` | `T` | `printLine` | [x] |

## Undefined runtime symbols

The complete `nm -D` output also contains these runtime imports. They are not
library API definitions and are supplied by libc or the ELF toolchain:

| symbol | type |
|---|---|
| `_ITM_deregisterTMCloneTable` | weak undefined |
| `_ITM_registerTMCloneTable` | weak undefined |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined |
| `__gmon_start__` | weak undefined |
| `atoi@GLIBC_2.2.5` | undefined libc |
| `fgets@GLIBC_2.2.5` | undefined libc |
| `printf@GLIBC_2.2.5` | undefined libc |
| `puts@GLIBC_2.2.5` | undefined libc |
| `stdin@GLIBC_2.2.5` | undefined libc |

Completion criterion: every defined API row is checked and the defined-symbol
set difference from C to Rust is empty.

Verified with sorted `comm` in both directions against
`target/release/libdriver.so`: zero missing symbols and zero extra symbols.
