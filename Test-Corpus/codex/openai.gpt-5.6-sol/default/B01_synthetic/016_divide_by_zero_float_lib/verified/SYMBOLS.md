# Dynamic Symbol Surface

Generated from:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
```

Only symbols defined by the C shared object are API exports. Undefined
`GLIBC` symbols and toolchain weak symbols are runtime imports, not library
entry points.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `bad` | `T` | `bad` | [x] |
| `driver` | `T` | `driver` | [x] |
| `good` | `T` | `good` | [x] |
| `printIntLine` | `T` | `printIntLine` | [x] |
| `printLine` | `T` | `printLine` | [x] |

## Imported C Symbols

The complete unfiltered `nm -D` output also contains the runtime imports
`printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, `__cxa_finalize@GLIBC_2.2.5`,
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, and
`__gmon_start__`. These are not defined public API symbols.

## Parity

- [x] Missing C-defined symbols in Rust: 0
- [x] Undefined non-libc API symbols in Rust: 0
