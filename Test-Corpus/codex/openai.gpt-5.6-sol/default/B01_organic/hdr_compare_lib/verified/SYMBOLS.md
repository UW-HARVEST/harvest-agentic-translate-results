# Dynamic symbol surface

Source library: `../c_src/build/libharvest-work-C5pt1I.so`

Mechanically extracted with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-C5pt1I.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `hdr_compare` | `T` | `hdr_compare` | [x] |

The remaining names printed by plain `nm -D` for the C library are weak,
undefined toolchain/runtime imports (`_ITM_*`, `__cxa_finalize`, and
`__gmon_start__`), not symbols defined by the library.
