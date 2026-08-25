# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

The CMake target is an executable, so `libdriver_c.so` is built from the same
unchanged `c_src/src/main.c` translation unit with `cc -shared -fPIC`.

| C symbol | type | C definition | Rust status | verified |
|----------|------|--------------|-------------|----------|
| `main` | `T` | `c_src/src/main.c:36` | exported by `src/lib.rs` and differential-tested through `libloading` | [x] |
| `printLine` | `T` | `c_src/src/main.c:28` | exported by `src/lib.rs` and differential-tested through `libloading` | [x] |

Undefined dynamic symbols in the C object are libc/runtime imports, not library
exports: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `atoi`, `fgets`, `memset`, `puts`, `stdin`,
and `strncpy`.
