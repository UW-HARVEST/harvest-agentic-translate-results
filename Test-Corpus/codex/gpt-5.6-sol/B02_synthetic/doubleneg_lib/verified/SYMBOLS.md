# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

Only definitions are library exports. The unfiltered C dynamic table also
contains weak toolchain hooks and undefined imports from libc/libm; those are
not C library exports.

| C symbol | C type | Rust type | Rust export | Status |
|----------|--------|-----------|-------------|--------|
| `calculate_with_doubles` | `T` | `T` | `calculate_with_doubles` | [x] |
| `convert_double_to_int` | `T` | `T` | `convert_double_to_int` | [x] |
| `create_numeric_buffer` | `T` | `T` | `create_numeric_buffer` | [x] |
| `doubleneg` | `T` | `T` | `doubleneg` | [x] |
| `find_value_in_buffer` | `T` | `T` | `find_value_in_buffer` | [x] |
| `process_negation` | `T` | `T` | `process_negation` | [x] |

Missing C exports in Rust: **0**

Undefined non-libc/non-libm symbols required by C: **0**
