# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-uQuYXX.so
```

| C symbol | C type | Rust export present | Rust type |
|----------|--------|---------------------|-----------|
| `call_predict` | `T` | [x] | `T` |

The C shared object exports no other defined dynamic symbols. In particular,
`get_predict_func` is declared in `include/lib.h` but has no definition in the
C source and is not present in the C dynamic symbol table.

## Completion checks

- [x] Every defined dynamic C symbol has an exact-name Rust export.
- [x] Missing C symbols from Rust: 0.
- [x] Undefined non-runtime/non-libc C symbols from Rust: 0.
