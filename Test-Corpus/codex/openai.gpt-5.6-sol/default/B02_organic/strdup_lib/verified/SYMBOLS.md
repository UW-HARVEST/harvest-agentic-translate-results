# Dynamic Symbol Surface

Source of truth:

```text
nm -D --defined-only ../c_src/build/libdriver.so
0000000000001129 T custom_strdup
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `custom_strdup` | `T` (global function) | `custom_strdup` | present |

The unfiltered C `nm -D` output also contains undefined libc/toolchain imports:
`strlen`, `memcpy`, `malloc`, `__cxa_finalize`, `__gmon_start__`,
`_ITM_deregisterTMCloneTable`, and `_ITM_registerTMCloneTable`. They are not
definitions exported by this library.

Completion:

- [x] Every dynamic symbol defined by the C shared library is defined by the
  Rust shared library with the exact same name.
- [x] Missing C definitions from Rust: none.
