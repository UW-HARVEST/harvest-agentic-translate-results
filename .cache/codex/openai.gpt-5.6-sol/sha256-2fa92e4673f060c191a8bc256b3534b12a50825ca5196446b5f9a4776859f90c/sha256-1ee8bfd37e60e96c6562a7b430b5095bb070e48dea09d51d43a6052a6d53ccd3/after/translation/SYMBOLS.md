# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Inventory command:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
```

| # | C symbol | Type | Rust export |
|---|----------|------|-------------|
| 1 | `bad` | `T` | present |
| 2 | `driver` | `T` | present |
| 3 | `good` | `T` | present |
| 4 | `printIntPtrLine` | `T` | present |

The complete C `nm -D` output also contains these undefined or weak runtime
dependencies; they are not definitions exported by this library:

| Symbol | C type |
|--------|--------|
| `_ITM_deregisterTMCloneTable` | `w` |
| `_ITM_registerTMCloneTable` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` |
| `__gmon_start__` | `w` |
| `printf@GLIBC_2.2.5` | `U` |

Current defined-symbol difference:

```text
(empty)
```

## Completion Gate

- [x] Every C-defined dynamic symbol is defined by the Rust shared object.
- [x] `ldd -r` reports no unresolved symbols in the Rust shared object.
