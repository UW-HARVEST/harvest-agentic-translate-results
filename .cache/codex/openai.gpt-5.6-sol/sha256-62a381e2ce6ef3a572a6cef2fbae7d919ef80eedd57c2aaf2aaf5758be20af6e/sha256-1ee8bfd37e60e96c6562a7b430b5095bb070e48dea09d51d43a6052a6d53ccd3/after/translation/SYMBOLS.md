# Dynamic Symbol Surface

Reference library: `../c_src/build/libdriver.so`

Command used:

```text
nm -D ../c_src/build/libdriver.so
```

## Defined public API symbols

| Symbol | C type | Rust export | Status |
|---|---|---|---|
| `driver` | `T` | `driver` | Present |

## Dynamic imports

These entries are not API exports implemented by this library.

| Symbol | C type | Classification |
|---|---|---|
| `_ITM_deregisterTMCloneTable` | `w` | ELF toolchain runtime |
| `_ITM_registerTMCloneTable` | `w` | ELF toolchain runtime |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc/runtime |
| `__gmon_start__` | `w` | ELF toolchain runtime |
| `printf@GLIBC_2.2.5` | `U` | libc |
| `puts@GLIBC_2.2.5` | `U` | libc |

## Completion

- [x] Every C-defined public symbol is exported by the Rust shared library.
- [x] Zero C-defined public symbols are missing from the Rust shared library.
- [x] Zero undefined non-libc API symbols require a Rust implementation.
