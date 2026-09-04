# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-L8zxXO.so
nm -D --defined-only target/release/libfindrep_lib.so
```

The C shared library has eight globally defined dynamic symbols. All eight are
functions and all eight have an exact-name Rust export.

| C symbol | C type | Rust symbol present |
|---|---:|:---:|
| `add_to_accumulator` | `T` | yes |
| `divide_multiplier` | `T` | yes |
| `find_and_replace_char` | `T` | yes |
| `findrep` | `T` | yes |
| `multiply_with_multiplier` | `T` | yes |
| `process_octal_string` | `T` | yes |
| `subtract_from_accumulator` | `T` | yes |
| `validate_and_normalize` | `T` | yes |

The C library's undefined dynamic symbols are libc/toolchain imports, not
library API: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `memchr`, `sprintf`, `strcpy`, and
`strlen`.

Missing C-defined symbols in Rust: **0**.

The integration test also resolves every symbol from both libraries through
`libloading` and repeats the `nm -D --defined-only` subset comparison.
