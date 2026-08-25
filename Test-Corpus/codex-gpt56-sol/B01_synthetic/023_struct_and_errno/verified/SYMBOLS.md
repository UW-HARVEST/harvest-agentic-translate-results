# Dynamic Symbol Surface

The CMake target is an executable, so the loadable C artifact was produced
from the unchanged source with:

```sh
cc -shared -fPIC -o c_src/build/libdriver_c.so c_src/src/main.c
```

Defined public symbols were extracted mechanically with
`nm -D --defined-only c_src/build/libdriver_c.so`.

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `main` | `T` | `main` | Present |
| `run` | `T` | `run` | Present |

The remaining raw `nm -D` entries are undefined or weak libc/toolchain
dependencies: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__errno_location`, `__gmon_start__`, `fgets`, `printf`,
`puts`, `stdin`, and `strtol`. They are not definitions supplied by this
library.

Missing C definitions in Rust: **0**.
