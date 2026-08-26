# Exported Symbol Surface

Source binary: `c_src/build/libtranslated_rust.so`

Extraction command:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

Only globally defined dynamic symbols are part of this table. The C library's
weak runtime symbols and libc imports are undefined and therefore excluded.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `apply_bitmask` | `T` | `apply_bitmask` | present |
| 2 | `arity` | `T` | `arity` | present |
| 3 | `arity2` | `T` | `arity2` | present |
| 4 | `arity3` | `T` | `arity3` | present |
| 5 | `arity4` | `T` | `arity4` | present |
| 6 | `compare_allocations` | `T` | `compare_allocations` | present |
| 7 | `init_matrix` | `T` | `init_matrix` | present |
| 8 | `process_string` | `T` | `process_string` | present |
| 9 | `shift_array` | `T` | `shift_array` | present |

Missing C symbols in Rust: **0**

Undefined non-runtime/non-libc symbols in Rust: **0**

