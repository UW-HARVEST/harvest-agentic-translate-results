# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

The CMake target is an executable, so `libdriver_c.so` is built from the same
`c_src/src/main.c` translation unit with `cc -fPIC -shared`.

| symbol | C type | Rust export | parity |
|--------|--------|-------------|--------|
| `bad` | `T` | `bad` | [x] |
| `good` | `T` | `good` | [x] |
| `main` | `T` | `main` | [x] |
| `printLine` | `T` | `printLine` | [x] |

Undefined C symbols are limited to libc/runtime symbols (`puts`,
`__cxa_finalize`) and weak toolchain hooks (`_ITM_*`, `__gmon_start__`).

Completion: [x] zero missing or undefined non-libc symbols in Rust
