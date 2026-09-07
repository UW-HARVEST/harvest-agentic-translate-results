# Dynamic symbol surface

Source binary: `c_src/build/libharvest-work-PBWdNu.so`

Command: `nm -D --defined-only`

| C symbol | Type | Rust export present |
|----------|------|---------------------|
| `apply_bit_operations` | `T` | [x] |
| `envy` | `T` | [x] |
| `init_config_from_env` | `T` | [x] |
| `parse_env_numeric` | `T` | [x] |
| `perform_operation` | `T` | [x] |

The C library's undefined dynamic symbols are libc/toolchain symbols:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, `atoi`, `fprintf`, `getenv`, `printf`, `puts`, `snprintf`,
`stderr`, and `strchr`.

Missing C-defined symbols in the Rust library: **0**.
