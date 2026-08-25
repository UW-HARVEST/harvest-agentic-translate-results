# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --format=posix c_src/build/libtranslated_rust.so
nm -D --defined-only --format=posix target/release/libpinflate_lib.so
```

The C shared object has eight defined public symbols. The CMake build has no
options or conditional compilation, and `Cargo.toml` has no `[features]`
table, so this is the symbol surface for the sole build configuration.

| C symbol | Kind | Size (bytes) | Rust export | Status |
|----------|------|-------------:|-------------|--------|
| `cp_dist_base` | data | 128 | `cp_dist_base` | [x] |
| `cp_dist_extra_bits` | data | 32 | `cp_dist_extra_bits` | [x] |
| `cp_error_reason` | BSS data | 8 | `cp_error_reason` | [x] |
| `cp_fixed_table` | data | 320 | `cp_fixed_table` | [x] |
| `cp_len_base` | data | 124 | `cp_len_base` | [x] |
| `cp_len_extra_bits` | data | 31 | `cp_len_extra_bits` | [x] |
| `cp_permutation_order` | data | 19 | `cp_permutation_order` | [x] |
| `pinflate` | function | implementation-dependent | `pinflate` | [x] |

The C object's undefined dynamic symbols are only libc/toolchain symbols:
`free`, `__assert_fail`, `memset`, `calloc`, `memcpy`, weak `_ITM_*`,
`__gmon_start__`, and `__cxa_finalize`. There are no undefined project
symbols.

**Result:** [x] zero C symbols missing from Rust.
