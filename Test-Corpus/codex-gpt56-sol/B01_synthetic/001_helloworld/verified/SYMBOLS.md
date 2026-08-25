# Dynamic Symbol Surface

Source oracle: `c_src/build/libdriver_c.so`, built from the unchanged
`c_src/src/main.c` with position-independent code.

Inventory command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

| C symbol | Type | Rust parity |
|----------|------|-------------|
| `main` | `T` (global text) | [x] Exported from the Rust `cdylib` with the exact name |

The C shared object has no other defined dynamic symbols.

- [x] Missing-symbol diff is empty.
- [x] Rust has no unresolved non-system symbols.
