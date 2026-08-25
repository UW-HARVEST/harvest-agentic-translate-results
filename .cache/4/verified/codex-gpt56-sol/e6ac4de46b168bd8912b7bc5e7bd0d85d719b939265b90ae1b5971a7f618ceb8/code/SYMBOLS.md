# Dynamic Symbol Surface

Derived from:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libenvy_lib.so
```

| C symbol | C type | Rust type | Rust status |
|----------|--------|-----------|-------------|
| `apply_bit_operations` | `T` | `T` | exported |
| `envy` | `T` | `T` | exported |
| `init_config_from_env` | `T` | `T` | exported |
| `parse_env_numeric` | `T` | `T` | exported |
| `perform_operation` | `T` | `T` | exported |

Missing C-defined symbols in Rust: **0**

The C library's undefined dynamic symbols are libc/toolchain symbols:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, `atoi`, `fprintf`, `getenv`, `printf`, `puts`, `snprintf`,
`stderr`, and `strchr`.
