# Dynamic Symbol Surface

Derived mechanically with:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

Only globally defined dynamic symbols are library API symbols. Undefined
entries (`puts`) and toolchain weak symbols are not implementations supplied by
the C library.

| C symbol | C type | Rust `.so` status |
|----------|--------|-------------------|
| `bad` | `T` | present |
| `driver` | `T` | present |
| `good` | `T` | present |
| `printLine` | `T` | present |

Missing C symbols in Rust: **0**

- [x] Final `nm -D` comparison has zero missing defined symbols.
- [x] The complete C `nm -D` name set is a subset of the Rust name set.
