# Dynamic Symbol Surface

Generated from the default C shared library:

```text
nm -D --defined-only c_src/build/libdriver.so
```

## C-defined public symbols

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `bad` | `T` | `bad` | present |
| `driver` | `T` | `driver` | present |
| `good` | `T` | `good` | present |
| `printIntPtrLine` | `T` | `printIntPtrLine` | present |

Missing C-defined symbols: **0**.

## C undefined/runtime symbols

The complete undefined portion of `nm -D c_src/build/libdriver.so` is:

| symbol | binding |
|--------|---------|
| `printf@GLIBC_2.2.5` | global undefined |
| `_ITM_deregisterTMCloneTable` | weak undefined |
| `_ITM_registerTMCloneTable` | weak undefined |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined |
| `__gmon_start__` | weak undefined |

These are runtime/toolchain imports, not library API definitions. The Rust
library imports `printf@GLIBC_2.2.5` and has no undefined non-runtime `driver`
library symbols.

## Completion

- [x] Final Phase D comparison confirms zero missing C-defined symbols for
  every feature combination.
- [x] Final Phase D comparison confirms zero undefined non-runtime library
  symbols.
