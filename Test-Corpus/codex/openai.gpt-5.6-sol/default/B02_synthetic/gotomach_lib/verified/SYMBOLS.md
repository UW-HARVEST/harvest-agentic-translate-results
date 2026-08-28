# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-wommeA.so
nm -D --defined-only target/release/libgotomach_lib.so
```

## Defined public symbols

| C symbol | C type | Rust type | Status |
|----------|--------|-----------|--------|
| `double_value` | `T` | `T` | [x] |
| `gotomach` | `T` | `T` | [x] |
| `process_value` | `T` | `T` | [x] |
| `triple_value` | `T` | `T` | [x] |

Missing from Rust: **0**

## Undefined C dynamic symbols

The C shared object imports these libc/toolchain symbols; none is a missing
library implementation:

| Symbol | Binding |
|--------|---------|
| `_ITM_deregisterTMCloneTable` | weak |
| `_ITM_registerTMCloneTable` | weak |
| `__cxa_finalize@GLIBC_2.2.5` | weak |
| `__gmon_start__` | weak |
| `free@GLIBC_2.2.5` | global |
| `malloc@GLIBC_2.2.5` | global |
| `puts@GLIBC_2.2.5` | global |

Undefined non-libc library symbols: **0**
