# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `UTIL_createLinePointers` | `T` | `UTIL_createLinePointers` | present |

The C shared object exports one defined public dynamic symbol. The Rust shared
object exports the same symbol with the exact name.

Undefined non-libc symbols missing from Rust: none.
