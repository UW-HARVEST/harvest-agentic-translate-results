# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-IP5DS8.so
nm -D --defined-only target/release/libfindrep_lib.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `add_to_accumulator` | `T` | `add_to_accumulator` | [x] |
| `divide_multiplier` | `T` | `divide_multiplier` | [x] |
| `find_and_replace_char` | `T` | `find_and_replace_char` | [x] |
| `findrep` | `T` | `findrep` | [x] |
| `multiply_with_multiplier` | `T` | `multiply_with_multiplier` | [x] |
| `process_octal_string` | `T` | `process_octal_string` | [x] |
| `subtract_from_accumulator` | `T` | `subtract_from_accumulator` | [x] |
| `validate_and_normalize` | `T` | `validate_and_normalize` | [x] |

The remaining C `nm -D` entries are ELF toolchain weak symbols or GLIBC
imports (`memchr`, `sprintf`, `strcpy`, and `strlen`), not public symbols
defined by this library.

Missing C-defined symbols in Rust: **0**
