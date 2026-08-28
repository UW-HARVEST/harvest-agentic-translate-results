# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-xlkWFI.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `to_barycentric` | `T` | `to_barycentric` | present |

The C library has no other defined dynamic symbols. Its remaining `nm -D`
entries (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, and `__gmon_start__`) are weak undefined toolchain symbols,
not library exports.

- [x] Final `nm -D --defined-only` symbol diff is empty.
- [x] Rust has no undefined non-system symbols.
