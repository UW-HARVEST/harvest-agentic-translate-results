# Dynamic symbol surface

Derived with:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

The CMake target is an executable, so `libdriver_c.so` is compiled from the
same unchanged `c_src/src/main.c` translation unit with `cc -shared -fPIC`.

| C address | type | symbol | Rust `.so` | status |
|-----------|------|--------|------------|--------|
| `0x1129` | `T` | `driver` | `T driver` | [x] |
| `0x115f` | `T` | `main` | `T main` | [x] |

Missing C-defined dynamic symbols in Rust: **0**.
Undefined application/library symbols in Rust: **0**. All undefined dynamic
references are loader-resolved platform runtime imports from `libc.so.6` or
`libgcc_s.so.1`, plus standard weak toolchain hooks.
