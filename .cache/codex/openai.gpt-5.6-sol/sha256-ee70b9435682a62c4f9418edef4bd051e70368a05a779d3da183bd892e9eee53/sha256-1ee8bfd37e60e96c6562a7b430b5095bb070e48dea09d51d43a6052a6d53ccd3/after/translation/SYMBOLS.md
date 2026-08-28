# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-Nek5bk.so
```

| C symbol | C address/type | Rust export | Status |
|----------|----------------|-------------|--------|
| `rev16` | `00000000000010f9 T` | `rev16` | [x] present |

The C library exports one public dynamic symbol. The release Rust library
exports the same symbol with the exact name.
