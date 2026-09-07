# Dynamic Symbol Surface

Derived from:

```text
nm -D c_src/build/libharvest-work-OIz0ME.so
nm -D --defined-only c_src/build/libharvest-work-OIz0ME.so
nm -D --defined-only translation/target/release/libwcscat_lib.so
```

## Defined public API symbols

| C symbol | C type | Rust symbol present | Required action |
|----------|--------|---------------------|-----------------|
| `wcscat` | `T` | yes (`T`) | none |

## Undefined runtime symbols shown by `nm -D`

These are compiler/runtime imports rather than library API exports, so they do
not require matching Rust exports.

| Symbol | C type | Classification |
|--------|--------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | weak toolchain runtime import |
| `_ITM_registerTMCloneTable` | `w` | weak toolchain runtime import |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | weak libc runtime import |
| `__gmon_start__` | `w` | weak toolchain runtime import |

## Completion

- [x] Final `nm -D --defined-only` symbol diff is empty.
- [x] Rust has no missing/undefined non-libc C API symbols.
