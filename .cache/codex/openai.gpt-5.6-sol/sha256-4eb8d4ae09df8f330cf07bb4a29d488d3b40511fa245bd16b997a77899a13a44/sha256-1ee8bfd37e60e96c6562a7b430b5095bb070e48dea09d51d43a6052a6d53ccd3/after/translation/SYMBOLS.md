# Dynamic Symbol Surface

Source: `nm -D ../c_src/build/libdriver.so`, built from the unmodified C source.

| C symbol | C type | Rust `nm -D` status | Classification |
|----------|--------|---------------------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | present (`w`) | undefined toolchain weak symbol |
| `_ITM_registerTMCloneTable` | `w` | present (`w`) | undefined toolchain weak symbol |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | present (`w`) | undefined libc weak symbol |
| `__gmon_start__` | `w` | present (`w`) | undefined toolchain weak symbol |
| `driver` | `T` | present (`T`) | defined public API |
| `printf@GLIBC_2.2.5` | `U` | present (`U`) | undefined libc symbol |

## Completion

- [x] Every C-defined public symbol is defined by the Rust shared library.
- [x] Missing C-defined public symbols: 0.
- [x] Missing/undefined non-libc API symbols in Rust: 0.
