# Dynamic Symbol Surface

Source: `nm -D c_src/build/libhello.so`, built from the unmodified C source.
Only symbols defined by the library are API exports; undefined entries are
dynamic runtime imports.

## Defined Public Symbols

| # | symbol | C type | Rust export |
|---|--------|--------|-------------|
| 1 | `helloworld` | `T` | present (`T`) |

## Dynamic Imports

The C library imports `puts@GLIBC_2.2.5`. Its weak toolchain/runtime entries
are `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`. These are not public API
symbols defined by the library.

## Parity

- [x] Every public symbol defined by the C library is defined by the Rust
  library with the exact same name.
- [x] Missing public symbols: 0.
