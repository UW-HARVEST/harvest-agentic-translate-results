# Dynamic Symbol Surface

Derived from:

```text
nm -D c_src/build/libtranslated_rust.so
```

| C `nm -D` type | symbol | Rust `nm -D` status |
|---|---|---|
| `w` | `_ITM_deregisterTMCloneTable` | present (`w`) |
| `w` | `_ITM_registerTMCloneTable` | present (`w`) |
| `w` | `__cxa_finalize@GLIBC_2.2.5` | present (`w`) |
| `w` | `__gmon_start__` | present (`w`) |
| `T` | `colourblind` | present (`T`) |

Defined public API symbols:

```text
colourblind
```

- [x] Every defined public C symbol is defined by the Rust shared library.
- [x] No defined public C symbol is missing from the Rust shared library.
- [x] The C shared library has no undefined non-runtime library symbol.

