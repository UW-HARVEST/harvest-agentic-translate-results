# Dynamic Symbol Surface

Source command:

```text
nm -D c_src/build/libdriver_c.so
```

## Library-defined public symbols

| symbol | C type | Rust parity |
|--------|--------|-------------|
| `bad` | `T` | exported |
| `good` | `T` | exported |
| `main` | `T` | exported |
| `printIntLine` | `T` | exported |
| `printLine` | `T` | exported |

The translated implementations originally existed only as private functions
in the Rust binary. `src/lib.rs` now exposes those implementations through
exact-name `extern "C"` wrappers. The defined-symbol diff is empty:

```text
comm -3 \
  <(nm -D --defined-only c_src/build/libdriver_c.so | awk '{print $3}' | sort) \
  <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort)
```

## Runtime and toolchain symbols

These are undefined or weak dynamic dependencies, not symbols implemented by
the library.

| symbol | C type | provider |
|--------|--------|----------|
| `_ITM_deregisterTMCloneTable` | `w` | compiler runtime |
| `_ITM_registerTMCloneTable` | `w` | compiler runtime |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc |
| `__gmon_start__` | `w` | compiler runtime |
| `__isoc99_scanf@GLIBC_2.7` | `U` | libc |
| `printf@GLIBC_2.2.5` | `U` | libc |
| `puts@GLIBC_2.2.5` | `U` | libc |
