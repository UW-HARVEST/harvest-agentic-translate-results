# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libSimpleList.so
```

## Library-owned public symbols

`nm -D --defined-only` reports one C export.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `smallestValue` | `T` | `smallestValue` | [x] |

## Imported toolchain symbols

These undefined weak symbols are emitted by the C compiler/runtime and are not
library API exports:

```text
_ITM_deregisterTMCloneTable
_ITM_registerTMCloneTable
__cxa_finalize@GLIBC_2.2.5
__gmon_start__
```

The C shared library has no undefined non-libc library symbols.
