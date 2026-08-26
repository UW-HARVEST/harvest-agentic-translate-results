# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --extern-only c_src/build/libtranslated_rust.so
```

Weak and undefined runtime/libc imports are not public symbols defined by the
C library.

| C symbol | C type | Rust symbol present | Source declaration |
|----------|--------|---------------------|--------------------|
| `encode_quant` | `T` | [x] | `c_src/include/lib.h:1` |

Missing C symbols in the Rust shared library: **0**.

Undefined non-runtime symbols in the Rust shared library: **0**.
