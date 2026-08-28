# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-heRBy4.so
```

The C shared object has 10 defined public dynamic symbols. The Rust status was
checked against `target/release/libcheckshift_lib.so`.

| # | C symbol | type | Rust export |
|---|----------|------|-------------|
| 1 | `add_with_static` | `T` | [x] |
| 2 | `apply_operation` | `T` | [x] |
| 3 | `checkshift` | `T` | [x] |
| 4 | `compute_checksum` | `T` | [x] |
| 5 | `execute_operation` | `T` | [x] |
| 6 | `get_operation` | `T` | [x] |
| 7 | `init_state` | `T` | [x] |
| 8 | `multiply_with_static` | `T` | [x] |
| 9 | `shift_with_static` | `T` | [x] |
| 10 | `xor_operation` | `T` | [x] |

Undefined C dynamic symbols are libc/toolchain dependencies, not library API:
`free`, `malloc`, `memcpy`, `printf`, `puts`, `__cxa_finalize`,
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, and
`__gmon_start__`.

Missing C API symbols in Rust: **0**
