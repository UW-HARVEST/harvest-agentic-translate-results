# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Command: `nm -D ../c_src/build/libdriver.so`

| Symbol | C kind | Rust `nm -D` parity | Classification |
|--------|--------|---------------------|----------------|
| `_ITM_deregisterTMCloneTable` | weak undefined | present | toolchain runtime |
| `_ITM_registerTMCloneTable` | weak undefined | present | toolchain runtime |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | present | libc runtime |
| `__gmon_start__` | weak undefined | present | toolchain runtime |
| `driver` | defined global (`T`) | present as defined global (`T`) | public API |
| `printf@GLIBC_2.2.5` | undefined | present | libc dependency |
| `putchar@GLIBC_2.2.5` | undefined | present | libc dependency |

## Defined Public API

| Symbol | C | Rust | Missing |
|--------|---|------|---------|
| `driver` | `T` | `T` | no |

- [x] Zero C-defined public symbols are missing from the Rust shared library.
- [x] Zero C non-libc dynamic dependencies are absent from the Rust dynamic
  symbol table.
