# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `driver` | `T` | `driver` (`T`) | present |

The Rust library has no missing C dynamic symbols.

- [x] C-to-Rust dynamic symbol diff is empty.
- [x] Rust has no unresolved project API symbols; its undefined dynamic
  symbols are satisfied by its listed `libc` and `libgcc_s` dependencies.
