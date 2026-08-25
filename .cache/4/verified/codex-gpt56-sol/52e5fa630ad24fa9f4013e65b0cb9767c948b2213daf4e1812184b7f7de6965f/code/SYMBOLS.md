# Dynamic Symbol Surface

Reference artifact:
`c_src/build/libdriver.so`, linked from CMake's position-independent
`CMakeFiles/driver.dir/src/container_of.c.o`.

Command:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | C type | Rust implementation | Rust export |
|----------|--------|---------------------|-------------|
| `find_container_of_a` | `T` | `src/lib.rs` | `find_container_of_a` |
| `find_container_of_b` | `T` | `src/lib.rs` | `find_container_of_b` |
| `main` | `T` | `src/lib.rs` | `main` |

The pre-Phase-A Rust crate was binary-only, so all three C symbols were
initially absent from a Rust shared object. The C implementation was translated
into `src/lib.rs`; no symbol is a stub.

Undefined C dynamic dependencies from `nm -D --undefined-only` are the libc
functions `atoi`, `memset`, and `printf`, plus the toolchain weak symbols
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
and `__gmon_start__`.

Completion: [x] all three symbols exist in the Rust shared object and the
defined-symbol diff is empty.
