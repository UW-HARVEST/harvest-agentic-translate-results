# Dynamic Symbol Surface

Source library: `c_src/build/libdriver_c.so`, compiled from the unchanged
`c_src/src/main.c` with `cc -shared -fPIC`.

Inventory command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

| symbol | C type | Rust `.so` status |
|--------|--------|-------------------|
| `driver` | `T` | present |
| `main` | `T` | present |

There are no macro-generated exports and no C-defined symbol is missing from
`target/debug/libdriver.so`.

The C library's undefined dynamic symbols are all libc/toolchain imports:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, `fgets`, `printf`, `stdin`, `strcspn`, and `strlen`.

- [x] C-defined exports missing from Rust: 0
- [x] Undefined non-libc/non-toolchain symbols in Rust: 0
