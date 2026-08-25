# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `allocate_and_compute` | `T` | `allocate_and_compute` | [x] |
| `fallcalc` | `T` | `fallcalc` | [x] |
| `foreach_sum` | `T` | `foreach_sum` | [x] |
| `process_array_reverse` | `T` | `process_array_reverse` | [x] |
| `safe_double_to_int` | `T` | `safe_double_to_int` | [x] |
| `switch_fallthrough_calculator` | `T` | `switch_fallthrough_calculator` | [x] |

The complete `nm -D` output also contains weak toolchain symbols
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
and `__gmon_start__`, plus the undefined libc imports `free` and `malloc`.
Those are runtime/toolchain dependencies rather than symbols defined by the C
library.

Missing C-defined symbols in the Rust shared library: **0**.
