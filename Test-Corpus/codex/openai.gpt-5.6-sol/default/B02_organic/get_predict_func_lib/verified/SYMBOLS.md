# Dynamic Symbol Surface

Reference library:
`../c_src/build/libharvest-work-5wUFnm.so`

Rust library:
`target/release/libget_predict_func_lib.so`

The table contains every globally defined public symbol reported by
`nm -D --defined-only` for the reference library.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `get_predict_func` | `T` | `get_predict_func` (`T`) | [x] |

Undefined weak C runtime symbols (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`) are not
library API definitions and are intentionally excluded.

Missing C symbols in Rust: **0**
