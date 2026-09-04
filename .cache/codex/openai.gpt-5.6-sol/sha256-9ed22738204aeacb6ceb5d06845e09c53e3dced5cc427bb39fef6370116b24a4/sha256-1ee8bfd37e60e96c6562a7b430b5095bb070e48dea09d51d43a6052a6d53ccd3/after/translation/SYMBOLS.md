# Dynamic Symbol Surface

Mechanically extracted with:

```text
nm -D ../c_src/build/libdriver.so
nm -D target/release/libdriver.so
```

The C library has one defined public API symbol. The remaining entries are
undefined or weak runtime references introduced by the C toolchain. All seven
C dynamic-symbol names are also present in the Rust shared library.

| C symbol | C binding | Classification | Rust `.so` |
|----------|-----------|----------------|-------------|
| `_ITM_deregisterTMCloneTable` | weak undefined | toolchain runtime | present |
| `_ITM_registerTMCloneTable` | weak undefined | toolchain runtime | present |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | libc/toolchain runtime | present |
| `__gmon_start__` | weak undefined | toolchain runtime | present |
| `div@GLIBC_2.2.5` | undefined | libc dependency | present |
| `driver` | defined global function | public API | present |
| `printf@GLIBC_2.2.5` | undefined | libc dependency | present |

## Defined public API parity

- [x] `driver`
- [x] Missing C-defined symbols in Rust: **0**
- [x] Missing C dynamic-symbol names in Rust, ignoring ELF version suffixes: **0**
- [x] Missing exact C dynamic-symbol names in Rust: **0**
- [x] Unresolved non-runtime relocations reported by `ldd -r`: **0**
- [x] Symbol parity holds for default and `--no-default-features` builds
