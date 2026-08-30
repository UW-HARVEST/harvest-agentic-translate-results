# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
nm -D target/release/libdriver.so
```

## C-defined public API

| symbol | C type | Rust type | Rust status |
|--------|--------|-----------|-------------|
| `driver` | `T` | `T` | present |

## Dynamic runtime entries

These entries appear in the complete C `nm -D` output but are weak runtime
references or imported libc symbols, not definitions supplied by this library.

| symbol | C type | Rust status |
|--------|--------|-------------|
| `_ITM_deregisterTMCloneTable` | `w` | present (`w`) |
| `_ITM_registerTMCloneTable` | `w` | present (`w`) |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | present (`w`) |
| `__gmon_start__` | `w` | present (`w`) |
| `printf@GLIBC_2.2.5` | `U` | present (`U`) |

## Completeness

- [x] Every C-defined dynamic symbol is exported by the Rust shared object.
- [x] Missing C-defined symbols: 0.
- [x] Undefined non-libc symbols required from Rust for C API parity: 0.
