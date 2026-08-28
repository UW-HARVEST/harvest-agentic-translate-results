# Dynamic Symbol Surface

Derived with:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-R81tG0.so
nm -D --defined-only target/release/libmathop_lib.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `add_operation` | `T` | `add_operation` | present |
| `allocate_results` | `T` | `allocate_results` | present |
| `divide_operation` | `T` | `divide_operation` | present |
| `get_computation_timestamp` | `T` | `get_computation_timestamp` | present |
| `get_operation_priority` | `T` | `get_operation_priority` | present |
| `is_valid_operation` | `T` | `is_valid_operation` | present |
| `mathop` | `T` | `mathop` | present |
| `modulo_operation` | `T` | `modulo_operation` | present |
| `multiply_operation` | `T` | `multiply_operation` | present |
| `perform_computation_with_history` | `T` | `perform_computation_with_history` | present |
| `select_operation` | `T` | `select_operation` | present |
| `subtract_operation` | `T` | `subtract_operation` | present |

The C library's strong undefined symbols are the libc functions `calloc`,
`printf`, and `time`. The Rust library resolves the same three functions
through libc. There are no missing C-defined symbols.

## Completion Gate

- [x] All 12 C-defined dynamic symbols are defined by the Rust shared object.
- [x] No C API symbol is undefined by the Rust shared object.
- [x] `ldd -r` reports no unresolved Rust relocations.
- [x] Symbol parity holds for the default and no-default-feature builds.
