# Dynamic Symbol Surface

Reference library: `../c_src/build/libharvest-work-9jqAfK.so`

The table is the complete output surface of `nm -D` on the C shared library.
`UND` entries are compiler/runtime imports, not library-owned exports.

| C symbol | C type | Rust type | Status |
|----------|--------|-----------|--------|
| `_ITM_deregisterTMCloneTable` | weak `UND` | weak `UND` | present |
| `_ITM_registerTMCloneTable` | weak `UND` | weak `UND` | present |
| `__cxa_finalize@GLIBC_2.2.5` | weak `UND` | weak `UND` | present |
| `__gmon_start__` | weak `UND` | weak `UND` | present |
| `crc16` | global defined function (`T`) | global defined function (`T`) | present |

## Library-Owned Export Gate

- [x] `crc16`
- [x] Zero C-defined dynamic symbols missing from Rust
- [x] Zero undefined non-libc library symbols in Rust
