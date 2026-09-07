# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-eYWaUB.so
nm -D --defined-only target/release/libdoubleneg_lib.so
```

The C shared object exports six global functions. The header declares only
`doubleneg`, but the remaining non-static source functions are also part of the
actual dynamic ABI and are therefore included.

| C symbol | Rust symbol | Status |
|---|---|---|
| `calculate_with_doubles` | `calculate_with_doubles` | [x] |
| `convert_double_to_int` | `convert_double_to_int` | [x] |
| `create_numeric_buffer` | `create_numeric_buffer` | [x] |
| `doubleneg` | `doubleneg` | [x] |
| `find_value_in_buffer` | `find_value_in_buffer` | [x] |
| `process_negation` | `process_negation` | [x] |

Missing C symbols in Rust: **0**.

Final verification was repeated after the default and `--no-default-features`
test runs. `ldd -r` reported no unresolved symbols for either shared object.
