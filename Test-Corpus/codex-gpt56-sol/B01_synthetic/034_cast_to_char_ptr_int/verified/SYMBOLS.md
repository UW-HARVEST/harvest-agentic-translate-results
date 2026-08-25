# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`, compiled from the unchanged
`c_src/src/main.c` with `cc -fPIC -shared`.

Mechanical inventory command:

```text
nm -D c_src/build/libdriver_c.so
```

| symbol | C type | classification | Rust parity |
|---|---:|---|---|
| `_ITM_deregisterTMCloneTable` | `w` | weak toolchain import | N/A: not a library API export |
| `_ITM_registerTMCloneTable` | `w` | weak toolchain import | N/A: not a library API export |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | weak libc import | N/A: resolved by libc |
| `__gmon_start__` | `w` | weak toolchain import | N/A: not a library API export |
| `__isoc99_scanf@GLIBC_2.7` | `U` | libc import | N/A: resolved by libc |
| `driver` | `T` | public API export | [x] Exported and differentially tested |
| `main` | `T` | public API export | [x] Exported and differentially tested |
| `printf@GLIBC_2.2.5` | `U` | libc import | N/A: resolved by libc |
| `putchar@GLIBC_2.2.5` | `U` | libc import | N/A: resolved by libc |

Completion criterion: the defined-symbol difference from C to Rust must be
empty. Undefined libc and weak compiler-runtime symbols are dependencies, not
API exports.

Verified Rust artifact: `target/debug/deps/libdriver.so`.
