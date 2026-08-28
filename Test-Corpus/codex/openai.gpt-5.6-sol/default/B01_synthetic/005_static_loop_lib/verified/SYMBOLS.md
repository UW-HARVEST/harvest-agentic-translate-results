# Dynamic Symbol Surface

Derived from:

```text
nm -D ../c_src/build/libStaticLoop.so
```

| C symbol | C type | Rust `.so` status | Classification |
|----------|--------|-------------------|----------------|
| `_ITM_deregisterTMCloneTable` | weak undefined | present, weak undefined | ELF runtime hook |
| `_ITM_registerTMCloneTable` | weak undefined | present, weak undefined | ELF runtime hook |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | present, weak undefined | libc runtime import |
| `__gmon_start__` | weak undefined | present, weak undefined | ELF runtime hook |
| `driver` | defined global text | present, defined global text | public API |
| `printf@GLIBC_2.2.5` | undefined | present, undefined | libc import |
| `static_sum` | defined global text | present, defined global text | public API |

Public API symbol diff (`nm -D --defined-only`) is empty.

- [x] Both C-defined public symbols are exported by the Rust shared object
      under the exact names.
- [x] The Rust shared object has no undefined non-runtime symbol inherited
      from the C API.
