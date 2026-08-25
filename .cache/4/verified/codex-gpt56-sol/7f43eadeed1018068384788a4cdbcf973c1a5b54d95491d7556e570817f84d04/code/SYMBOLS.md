# Dynamic Symbol Surface

Source of truth:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | C origin | Rust export | Status |
|----------|----------|-------------|--------|
| `bad` | `c_src/src/driver.c:43` | `src/lib.rs:28` | [x] |
| `driver` | `c_src/src/driver.c:90` | `src/lib.rs:71` | [x] |
| `good` | `c_src/src/driver.c:84` | `src/lib.rs:65` | [x] |
| `printHexCharLine` | `c_src/src/driver.c:38` | `src/lib.rs:21` | [x] |
| `printLine` | `c_src/src/driver.c:30` | `src/lib.rs:12` | [x] |

The public header declares only `driver`, but the other four functions have
external linkage and are present in the C dynamic symbol table, so they are
part of the tested ABI.

The full `nm -D` output also contains weak toolchain symbols
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
and `__gmon_start__`, plus the libc imports `printf` and `puts`. They are not
definitions supplied by this library and are excluded from export parity.

Missing C-defined symbols in Rust: **0**
