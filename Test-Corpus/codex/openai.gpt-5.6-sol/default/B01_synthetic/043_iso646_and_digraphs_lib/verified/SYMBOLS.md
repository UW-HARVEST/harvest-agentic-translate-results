# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Command used:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| symbol | C type | Rust `.so` status |
|--------|--------|-------------------|
| `driver` | `T` (global function) | present as `T driver` |

The unfiltered C `nm -D` output also contains only undefined libc/toolchain
references: `printf`, `puts`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`. These are
dynamic dependencies, not symbols exported by the C library.

Missing C exports in Rust: **0**

