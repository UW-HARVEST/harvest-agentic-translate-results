# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
```

## Library-defined public symbols

| symbol | C `nm` type | Rust export | status |
|--------|-------------|-------------|--------|
| `driver` | `T` | `driver` (`T`) | [x] |

## Imported runtime symbols

These are undefined runtime/toolchain imports, not API exports implemented by
this library.

| symbol | C `nm` type | Rust dynamic table |
|--------|-------------|--------------------|
| `_ITM_deregisterTMCloneTable` | `w` | present (`w`) |
| `_ITM_registerTMCloneTable` | `w` | present (`w`) |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | present (`w`) |
| `__gmon_start__` | `w` | present (`w`) |
| `printf@GLIBC_2.2.5` | `U` | present (`U`) |
| `puts@GLIBC_2.2.5` | `U` | present (`U`) |

The C shared object has one defined dynamic symbol. No C-defined public symbol
is missing from the Rust shared object.
