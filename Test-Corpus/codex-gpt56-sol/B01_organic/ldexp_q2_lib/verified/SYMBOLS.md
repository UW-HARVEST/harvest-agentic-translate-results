# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libtranslated_rust.so
```

## Defined public API

| symbol | C type | Rust `.so` status |
|--------|--------|-------------------|
| `ldexp_q2` | `T` (global function) | present |

## Undefined toolchain symbols

These are ELF startup/runtime imports, not library API:

| symbol | type |
|--------|------|
| `_ITM_deregisterTMCloneTable` | weak undefined |
| `_ITM_registerTMCloneTable` | weak undefined |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined, libc |
| `__gmon_start__` | weak undefined |

## Completion

- [x] The C and Rust shared libraries have identical defined public symbol sets.
- [x] Rust has no missing or undefined non-libc C API symbol.
