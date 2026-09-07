# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
nm -D --defined-only ../c_src/build/libdriver.so
```

## C dynamic symbols

| Symbol | C `nm -D` type | Classification | Rust parity |
|--------|----------------|----------------|-------------|
| `_ITM_deregisterTMCloneTable` | `w` | Toolchain weak undefined symbol | Not a library API; excluded from defined-symbol parity |
| `_ITM_registerTMCloneTable` | `w` | Toolchain weak undefined symbol | Not a library API; excluded from defined-symbol parity |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc weak undefined symbol | Resolved by libc |
| `__gmon_start__` | `w` | Toolchain weak undefined symbol | Not a library API; excluded from defined-symbol parity |
| `driver` | `T` | Public defined library symbol | Present as exact symbol `driver` |
| `printf@GLIBC_2.2.5` | `U` | libc undefined symbol | Resolved by libc |

## Defined public-symbol parity

| # | C symbol | Rust symbol | Status |
|---|----------|-------------|--------|
| 1 | `driver` | `driver` | [x] |

Missing C-defined public symbols in Rust: **0**

Undefined non-libc C symbols requiring a Rust implementation: **0**
