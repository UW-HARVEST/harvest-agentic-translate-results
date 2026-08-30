# Dynamic Symbol Surface

Reference library:

```text
c_src/build/libdriver.so
```

Mechanical source:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| # | symbol | C type | Rust status |
|---|--------|--------|-------------|
| 1 | `G_OP` | `D` | present |
| 2 | `G_OP_NAME` | `D` | present |
| 3 | `helper_call` | `T` | present |
| 4 | `helper_ptr` | `T` | present |
| 5 | `main` | `T` | present |
| 6 | `op_add` | `T` | present |
| 7 | `op_mul` | `T` | present |
| 8 | `op_sub` | `T` | present |
| 9 | `use_generated` | `T` | present |

`main` was initially missing because the translated behavior existed only as
the Rust executable entry point. It is now implemented as a real `extern "C"`
library export with the C argument contract and return values.

Undefined C-library dependencies (`atoi`, `fprintf`, `printf`, `stderr`, and
toolchain weak symbols) are not implementation symbols and are tracked only
when checking for unresolved non-libc dependencies.

Completion:

- [x] All 9 C implementation symbols are present in the Rust shared library.
- [x] The symbol difference is empty for all 24 valid feature combinations.
