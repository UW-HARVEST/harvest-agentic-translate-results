# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only c_src/build/libdriver.so
```

The CMake target is an executable, so `libdriver.so` is linked from CMake's
position-independent `main.c.o` without modifying `c_src/`.

| C address | type | symbol | Rust parity |
|-----------|------|--------|-------------|
| `0000000000001129` | `T` | `driver` | [x] exported by Rust `.so` |
| `0000000000001174` | `T` | `main` | [x] exported by Rust `.so` |

Undefined dynamic symbols in the C library are libc/runtime imports
(`__cxa_finalize`, `__isoc99_scanf`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__gmon_start__`, and `printf`), not public
library definitions.

Final comparison:

```text
comm -23 <(nm -D --defined-only c_src/build/libdriver.so ...) \
         <(nm -D --defined-only target/debug/libdriver.so ...)
# no output
```
