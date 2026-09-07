# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-b2y2gT.so
nm -D --defined-only target/release/libdequantize_granule_lib.so
```

| C symbol | C type | Rust symbol present | Rust type | Status |
|----------|--------|---------------------|-----------|--------|
| `dequantize_granule` | `T` | yes | `T` | [x] |

The weak undefined toolchain symbols in the C shared object
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, and `__gmon_start__`) are not library API exports.

## Missing C API symbols in Rust

None.

## Final parity gate

- [x] Re-run the symbol comparison after all fixes.
- [x] Confirm zero C-defined symbols are missing from Rust.
- [x] Confirm the Rust library has no undefined non-runtime project symbols.
