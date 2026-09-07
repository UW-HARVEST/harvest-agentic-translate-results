# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-subbLC.so
```

Undefined weak C runtime imports shown by `nm -D` are not library exports and
are excluded.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `colourblind` | `T` | `colourblind` | [x] present |

Missing Rust exports: **0**

Final Phase D verification:

- [x] Sorted `nm -D --defined-only` symbol sets are identical.
- [x] `ldd -r target/release/libcolourblind_lib.so` reports no unresolved
  symbols.
