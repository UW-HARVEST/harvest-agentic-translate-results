# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The undefined weak entries printed by plain `nm -D`
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, and `__gmon_start__`) are toolchain runtime imports, not
symbols defined by this library.

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `flip_horizontal` | `T` | `flip_horizontal` | [x] |

Missing defined C symbols in the Rust shared library: **0**.

