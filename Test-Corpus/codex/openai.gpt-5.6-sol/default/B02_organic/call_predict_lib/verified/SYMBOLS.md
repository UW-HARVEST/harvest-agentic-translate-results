# Dynamic Symbol Surface

Reference library:
`../c_src/build/libharvest-work-7iUyaQ.so`

Translation library:
`target/release/libcall_predict_lib.so`

The table is derived from `nm -D --defined-only` on the reference shared
library. Undefined weak runtime symbols are loader dependencies, not public
definitions, and are not part of the callable library API.

| C symbol | C type | Rust type | Status |
|----------|--------|-----------|--------|
| `call_predict` | `T` | `T` | present |

Missing C definitions in Rust: **0**

The public header declares `get_predict_func(int)`, but the C source does not
define it and the reference shared library does not export it. The implemented
external entry point is `call_predict(int)`.
