# Dynamic symbol surface

Source: `nm -D --defined-only ../c_src/build/libharvest-work-B6YCD0.so`.

| C symbol | Rust symbol | Status |
|----------|-------------|--------|
| `read_side_info` | `read_side_info` | [x] |

Missing C symbols in the Rust shared library: **0**.

There are no macro-generated public symbols and `get_bits` is `static` in C.
