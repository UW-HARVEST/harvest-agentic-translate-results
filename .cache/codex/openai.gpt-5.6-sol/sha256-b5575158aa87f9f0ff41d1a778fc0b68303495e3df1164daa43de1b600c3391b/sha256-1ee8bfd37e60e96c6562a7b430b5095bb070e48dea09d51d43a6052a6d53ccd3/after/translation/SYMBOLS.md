# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --extern-only ../c_src/build/libdriver.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `tool_basename` | `T` | `tool_basename` | Present |

The C library has no other defined dynamic symbols. Its only non-weak undefined
symbol is the libc function `strrchr`.

Completion:

- [x] Every defined C dynamic symbol is exported by the Rust shared library.
- [x] Missing symbols: 0.
- [x] Undefined non-libc symbols in Rust: 0.
