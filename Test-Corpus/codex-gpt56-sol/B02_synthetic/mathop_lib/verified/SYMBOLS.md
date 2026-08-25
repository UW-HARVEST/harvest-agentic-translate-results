# Dynamic Symbol Surface

Source command:

```sh
nm -D --defined-only --format=posix c_src/build/libtranslated_rust.so
```

The CMake default configuration has no compile definitions or options. The
Cargo manifest has no `[features]` table, so its only valid feature combination
is `--no-default-features`.

| # | C symbol | C type | Rust `.so` status |
|---|----------|--------|-------------------|
| 1 | `add_operation` | `T` | [x] exported |
| 2 | `allocate_results` | `T` | [x] exported |
| 3 | `divide_operation` | `T` | [x] exported |
| 4 | `get_computation_timestamp` | `T` | [x] exported |
| 5 | `get_operation_priority` | `T` | [x] exported |
| 6 | `is_valid_operation` | `T` | [x] exported |
| 7 | `mathop` | `T` | [x] exported |
| 8 | `modulo_operation` | `T` | [x] exported |
| 9 | `multiply_operation` | `T` | [x] exported |
| 10 | `perform_computation_with_history` | `T` | [x] exported |
| 11 | `select_operation` | `T` | [x] exported |
| 12 | `subtract_operation` | `T` | [x] exported |

Completion check:

- [x] C-to-Rust defined-symbol diff is empty.
- [x] Rust has no unresolved dependency on a C-library project symbol.

