# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-haOvWm.so`

The mechanically collected `nm -D` output contains one defined public API
symbol:

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `tritanopia` | `T` | `tritanopia` | present |

The remaining C dynamic symbols are runtime imports or weak toolchain hooks,
not library API exports:

| Symbol | C type | Classification |
|--------|--------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | weak toolchain hook |
| `_ITM_registerTMCloneTable` | `w` | weak toolchain hook |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | weak libc hook |
| `__gmon_start__` | `w` | weak toolchain hook |
| `pow@GLIBC_2.29` | `U` | libm import |

Missing defined C API symbols in Rust: **0**.
