# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-nJQRmM.so
nm -D --defined-only target/release/libarity_lib.so
```

## C-defined public symbols

| # | symbol | C type | Rust type | parity |
|---|--------|--------|-----------|--------|
| 1 | `apply_bitmask` | `T` | `T` | [x] |
| 2 | `arity` | `T` | `T` | [x] |
| 3 | `arity2` | `T` | `T` | [x] |
| 4 | `arity3` | `T` | `T` | [x] |
| 5 | `arity4` | `T` | `T` | [x] |
| 6 | `compare_allocations` | `T` | `T` | [x] |
| 7 | `init_matrix` | `T` | `T` | [x] |
| 8 | `process_string` | `T` | `T` | [x] |
| 9 | `shift_array` | `T` | `T` | [x] |

Missing C-defined symbols in Rust: **0**.

## C runtime imports

The unfiltered C `nm -D` output also contains these undefined runtime imports.
They are dependencies, not public symbols defined by the library:

| binding | symbol |
|---------|--------|
| `U` | `free@GLIBC_2.2.5` |
| `U` | `malloc@GLIBC_2.2.5` |
| `U` | `memmove@GLIBC_2.2.5` |
| `U` | `strlen@GLIBC_2.2.5` |
| `w` | `_ITM_deregisterTMCloneTable` |
| `w` | `_ITM_registerTMCloneTable` |
| `w` | `__cxa_finalize@GLIBC_2.2.5` |
| `w` | `__gmon_start__` |

All are libc/compiler-runtime symbols; the C library has no undefined
non-runtime project symbols.

