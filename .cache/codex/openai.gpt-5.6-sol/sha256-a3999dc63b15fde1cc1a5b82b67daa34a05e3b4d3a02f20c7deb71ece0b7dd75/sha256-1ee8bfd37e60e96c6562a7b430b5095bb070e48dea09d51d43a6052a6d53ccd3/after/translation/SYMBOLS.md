# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-Ha9tNx.so
nm -D --defined-only target/release/libtritanopia_lib.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `tritanopia` | `T` (global function) | `tritanopia` | [x] |

The C dynamic table also imports `pow@GLIBC_2.29` from libm and contains the
usual weak ELF runtime entries (`_ITM_*`, `__cxa_finalize`, and
`__gmon_start__`). These are dependencies rather than public definitions.
Rust likewise imports `pow@GLIBC_2.29`. There are no missing C-defined dynamic
symbols.

Final `comm` diff of C-defined symbols against Rust-defined symbols: empty.
