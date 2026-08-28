# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-UnWJLG.so
nm -D --defined-only target/release/libcolourblind_lib.so
```

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `colourblind` | `T` (global text) | `colourblind` | [x] Present |

The C library has no other defined dynamic symbols. Its undefined dynamic
symbols are the toolchain/runtime imports `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, and
`__gmon_start__`; they are not public library APIs.

Completion audit: [x] zero C API symbols are missing from the Rust library,
and `ldd -r target/release/libcolourblind_lib.so` reports no unresolved
relocations.
