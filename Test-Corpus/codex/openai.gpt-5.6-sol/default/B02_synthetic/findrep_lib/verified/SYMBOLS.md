# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-xr5AfX.so
nm -D --defined-only target/release/libfindrep_lib.so
```

Only defined dynamic symbols are API exports. Undefined C-runtime imports are
not library API symbols.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `add_to_accumulator` | `T` | `add_to_accumulator` | present |
| `divide_multiplier` | `T` | `divide_multiplier` | present |
| `find_and_replace_char` | `T` | `find_and_replace_char` | present |
| `findrep` | `T` | `findrep` | present |
| `multiply_with_multiplier` | `T` | `multiply_with_multiplier` | present |
| `process_octal_string` | `T` | `process_octal_string` | present |
| `subtract_from_accumulator` | `T` | `subtract_from_accumulator` | present |
| `validate_and_normalize` | `T` | `validate_and_normalize` | present |

Missing from Rust: **0**

The C library's undefined dynamic symbols are libc/toolchain imports
(`__cxa_finalize`, `__gmon_start__`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `memchr`, `sprintf`, `strcpy`, and `strlen`);
none is a library-defined API symbol.
