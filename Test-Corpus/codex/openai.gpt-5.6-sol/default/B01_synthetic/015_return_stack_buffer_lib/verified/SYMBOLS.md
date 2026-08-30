# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Extraction command:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
```

The C library has four globally defined dynamic symbols. Runtime weak symbols
and imported libc symbols shown by unfiltered `nm -D` are not library exports.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `bad` | `T` | `bad` | present |
| `driver` | `T` | `driver` | present |
| `good` | `T` | `good` | present |
| `printLine` | `T` | `printLine` | present |

- [x] Missing C-defined symbols in the Rust shared library: 0
- [x] Undefined non-runtime/non-libc C dependencies: 0
