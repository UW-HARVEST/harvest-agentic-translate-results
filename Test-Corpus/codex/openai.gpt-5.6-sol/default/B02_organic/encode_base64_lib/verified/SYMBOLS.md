# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | Type | Rust `.so` status |
|----------|------|-------------------|
| `encode_base64` | `T` (global text) | present with exact name |

The C library has no other defined dynamic symbols. Its undefined dynamic
references are only the libc/runtime symbols `calloc`, `strlen`,
`__cxa_finalize`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, and `__gmon_start__`.

- [x] Missing C-defined symbols in Rust: 0
- [x] Missing non-libc dependencies required by the C library: 0
- [x] `ldd -r target/release/libdriver.so` reports no unresolved symbols

Final Phase D comparison:

```text
C defined symbols: encode_base64
Missing from Rust:
Rust exact export: 0000000000011850 T encode_base64
```
