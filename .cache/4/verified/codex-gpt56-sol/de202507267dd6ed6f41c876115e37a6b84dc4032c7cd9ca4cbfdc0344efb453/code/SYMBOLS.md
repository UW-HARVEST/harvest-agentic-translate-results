# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The CMake configuration has one shared-library target and no build options or
preprocessor feature switches. `Cargo.toml` declares no features, so the only
Rust feature combination is the empty set (`--no-default-features`).

| C symbol | C type | Rust `.so` status |
|----------|--------|-------------------|
| `add_operation` | `T` | present |
| `multiply_operation` | `T` | present |
| `subtract_operation` | `T` | present |
| `modulo_operation` | `T` | present |
| `safe_double_to_int` | `T` | present |
| `compute_scaled_value` | `T` | present |
| `compare_results_in_array` | `T` | present |
| `init_result_array` | `T` | present |
| `process_with_foreach` | `T` | present |
| `compute_weighted_sum` | `T` | present |
| `arrayfunc` | `T` | present |

Missing C-defined dynamic symbols in Rust: **0**

Undefined entries shown by plain `nm -D` on the C library are ELF/runtime
imports (`_ITM_*`, `__cxa_finalize`, and `__gmon_start__`), not library API
definitions.

## Completion

- [x] Final `nm -D` comparison has zero C-defined symbols missing from Rust.
- [x] `ldd -r` reports no unresolved Rust-library relocations.
