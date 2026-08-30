# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | Type | C source | Rust export | Status |
|----------|------|----------|-------------|--------|
| `driver` | `T` | `include/driver.h`, `src/driver.c` | `extern "C" fn driver` | [x] |
| `print_foo` | `T` | `src/driver.c` | `extern "C" fn print_foo` | [x] |

The mechanically sorted defined-symbol sets are identical. The C library has
no other defined dynamic symbols:

```text
driver
print_foo
```

The C library's undefined dynamic symbols are the libc function `printf` and
weak ELF/toolchain hooks (`_ITM_*`, `__cxa_finalize`, and `__gmon_start__`).
There are no undefined project symbols.
