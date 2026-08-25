# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libtranslated_rust.so`.

| # | symbol | C type | C origin | Rust `.so` |
|---|--------|--------|----------|-------------|
| 1 | `convert_pix` | `T` (function) | `c_src/src/lib.c` | present |
| 2 | `cp_dist_base` | `D` (data) | `c_src/src/lib.c` | present |
| 3 | `cp_dist_extra_bits` | `D` (data) | `c_src/src/lib.c` | present |
| 4 | `cp_error_reason` | `B` (zero-initialized data) | `c_src/src/lib.c` | present |
| 5 | `cp_fixed_table` | `D` (data) | `c_src/src/lib.c` | present |
| 6 | `cp_inflate` | `T` (function) | `c_src/src/lib.c` | present |
| 7 | `cp_len_base` | `D` (data) | `c_src/src/lib.c` | present |
| 8 | `cp_len_extra_bits` | `D` (data) | `c_src/src/lib.c` | present |
| 9 | `cp_permutation_order` | `D` (data) | `c_src/src/lib.c` | present |

## Undefined Dependency Audit

The C library's undefined dynamic symbols are libc/runtime dependencies:
`__assert_fail`, `calloc`, `free`, `memcmp`, `memcpy`, and `memset`, plus the
weak toolchain hooks `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`. It has no
undefined project-library symbol.

## Parity Gate

- [x] The defined-symbol diff between the C and Rust shared libraries is empty.
- [x] All exported data symbols have byte-identical initial contents.
