# Exported Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-GY1cd5.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `max_size_frame` | `T` | `max_size_frame` | [x] present |

The C shared library exports one public symbol. The Rust shared library exports
the same symbol with the exact name.

- [x] `nm -D` shows 0 C symbols missing from the Rust shared library.
