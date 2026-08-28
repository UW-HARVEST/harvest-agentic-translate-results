# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libhello.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `helloworld` | `T` | `helloworld` | [x] Present |

The C shared library exports one defined public symbol. The Rust shared library
exports the same symbol with the exact name. The C library has no undefined
non-libc symbols.

- [x] `nm -D` shows 0 C symbols missing from the Rust shared library.
- [x] Rust has 0 undefined non-libc symbols required by the C API.
