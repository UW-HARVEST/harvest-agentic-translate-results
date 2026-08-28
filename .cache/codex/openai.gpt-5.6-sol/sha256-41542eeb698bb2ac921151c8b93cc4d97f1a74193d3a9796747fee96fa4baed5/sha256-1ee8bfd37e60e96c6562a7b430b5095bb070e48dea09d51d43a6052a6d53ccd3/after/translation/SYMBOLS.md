# Dynamic Symbol Surface

Source binary: `../c_src/build/libharvest-work-fHe7tI.so`

The table is generated from globally defined dynamic symbols reported by
`nm -D --defined-only`. The C library has no other defined public symbols.

| symbol | C `.so` | Rust `.so` | status |
|--------|----------|------------|--------|
| `calculate_with_doubles` | `T` | `T` | present |
| `convert_double_to_int` | `T` | `T` | present |
| `create_numeric_buffer` | `T` | `T` | present |
| `doubleneg` | `T` | `T` | present |
| `find_value_in_buffer` | `T` | `T` | present |
| `process_negation` | `T` | `T` | present |

Missing C symbols in Rust: **0**

Undefined non-libc C symbols missing from Rust: **0**

## Completion

- [x] Default feature build has all six exports.
- [x] `--no-default-features` build has all six exports.
- [x] Dynamic symbol diff is empty.
