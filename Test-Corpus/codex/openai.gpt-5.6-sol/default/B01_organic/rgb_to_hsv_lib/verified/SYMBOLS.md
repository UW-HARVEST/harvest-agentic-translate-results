# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only --extern-only ../c_src/build/libharvest-work-1dcjw8.so
nm -D --defined-only --extern-only target/release/librgb_to_hsv_lib.so
```

The unfiltered C `nm -D` output also contains the undefined weak toolchain
hooks `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`. They are imports, not
symbols defined by the C library.

| C symbol | C type | Rust symbol | Rust type | parity |
|----------|--------|-------------|-----------|--------|
| `rgb_to_hsv` | `T` | `rgb_to_hsv` | `T` | [x] |

Missing C-defined public symbols in Rust: **0**.
