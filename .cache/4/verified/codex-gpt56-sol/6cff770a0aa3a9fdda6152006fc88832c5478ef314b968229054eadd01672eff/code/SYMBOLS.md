# Dynamic Symbol Surface

Generated from the default C shared object with:

```text
nm -D --defined-only --format=posix c_src/build/libtranslated_rust.so
```

Only defined global API symbols (`T`, `D`, `B`, or `R`) are included. Undefined
GLIBC imports (`memset` and `sqrtf`) and toolchain weak symbols are not library
API exports.

| C symbol | kind | C declaration | Rust export | status |
|----------|------|---------------|-------------|--------|
| `normalize` | `T` | `void normalize(float *dest, const float *src, int size)` | `normalize` | [x] |

Missing C symbols in the Rust shared object: **0**.

