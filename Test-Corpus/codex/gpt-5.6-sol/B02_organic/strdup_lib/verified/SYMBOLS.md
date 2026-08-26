# Dynamic Symbol Surface

Generated from:

```text
nm -D --format=posix c_src/build/libdriver.so
```

The C library has one defined public symbol. The other dynamic-table entries
are undefined imports supplied by libc or the ELF toolchain. The Rust library
contains every C entry with the same name and symbol version where applicable.

| C dynamic symbol | C kind | Rust dynamic table | Notes |
|---|---:|---:|---|
| `_ITM_deregisterTMCloneTable` | weak undefined | present | ELF toolchain import |
| `_ITM_registerTMCloneTable` | weak undefined | present | ELF toolchain import |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | present | libc import |
| `__gmon_start__` | weak undefined | present | ELF toolchain import |
| `custom_strdup` | defined (`T`) | present, defined (`T`) | Public API |
| `malloc@GLIBC_2.2.5` | undefined | present | libc import |
| `memcpy@GLIBC_2.14` | undefined | present | libc import |
| `strlen@GLIBC_2.2.5` | undefined | present | libc import |

Defined-symbol comparison:

```text
C:    custom_strdup
Rust: custom_strdup
Missing from Rust: none
```

- [x] Every C-defined dynamic symbol is defined by the Rust shared library.
- [x] There are no missing or undefined non-libc public API symbols in Rust.
