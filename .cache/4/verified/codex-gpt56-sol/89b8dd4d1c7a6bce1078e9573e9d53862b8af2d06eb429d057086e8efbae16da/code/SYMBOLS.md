# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | present |
| `foo` | `T` | `foo` | present |

The C library has 2 defined dynamic symbols. The Rust library has both, with
the exact names. There are no missing C symbols.

- [x] `nm -D` symbol diff is empty.
- [x] Rust has no unresolved translated-library symbols; its undefined imports
  are platform C and Rust runtime dependencies.
