# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --extern-only ../c_src/build/libdriver.so
nm -D --defined-only --extern-only target/release/libdriver.so
```

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `searchAndReplace` | `T` | `searchAndReplace` (`T`) | [x] |

The C library's remaining dynamic symbols are undefined libc/toolchain imports:
`malloc`, `realloc`, `strdup`, `strlen`, `strncpy`, `strstr`,
`__cxa_finalize`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, and `__gmon_start__`. They are not public
symbols implemented by this library.

