# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-jeC2iM.so`

Rust library:
`target/release/libbitwriter_add_lib.so`

## Defined public symbols

| C symbol | C type | Rust type | Rust status |
|----------|--------|-----------|-------------|
| `bitwriter_add` | `T` | `T` | present |

Missing defined public symbols: **0**

## Complete C `nm -D` inventory

The remaining dynamic entries are weak, undefined toolchain/runtime symbols,
not public symbols implemented by this library.

| Symbol | Type | Classification |
|--------|------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | undefined weak runtime symbol |
| `_ITM_registerTMCloneTable` | `w` | undefined weak runtime symbol |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | undefined weak libc symbol |
| `__gmon_start__` | `w` | undefined weak runtime symbol |
| `bitwriter_add` | `T` | defined public API |

Undefined non-libc library symbols: **0**

- [x] Every public symbol defined by the C `.so` is defined by the Rust `.so`.
- [x] No C library symbol is left undefined by the Rust `.so`.
