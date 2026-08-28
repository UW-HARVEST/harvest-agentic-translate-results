# Dynamic Symbol Surface

Reference library: `../c_src/build/libStaticAlias.so`

Command:

```sh
nm -D ../c_src/build/libStaticAlias.so
```

## C-defined public exports

| symbol | C type | Rust export | status |
|---|---:|---:|---:|
| `driver` | `T` | `T` | present |
| `static_alias` | `T` | `T` | present |

## Complete remaining `nm -D` surface

These are undefined weak/runtime symbols rather than API definitions. They are
listed so that every line emitted by `nm -D` is accounted for.

| symbol | C type | role | Rust resolution |
|---|---:|---|---|
| `_ITM_deregisterTMCloneTable` | `w` | toolchain weak import | weak import |
| `_ITM_registerTMCloneTable` | `w` | toolchain weak import | weak import |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc weak import | weak import |
| `__gmon_start__` | `w` | toolchain weak import | weak import |
| `printf@GLIBC_2.2.5` | `U` | libc import used by `driver` | libc import |

## Completion

- [x] Every C-defined dynamic symbol is exported by the Rust `cdylib` under
      the exact same name.
- [x] There are zero missing C-defined symbols.
- [x] There are zero undefined non-libc application symbols.
