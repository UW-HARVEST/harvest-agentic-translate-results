# Dynamic Symbol Surface

Source: `nm -D c_src/build/libdriver.so`, where `libdriver.so` is linked from
the PIC object produced by the required CMake build.

## Defined public symbols

| symbol | C type | Rust status |
|--------|--------|-------------|
| `main` | `T` | present (`extern "C"`, unmangled) |

## Undefined runtime imports

These are libc/toolchain dependencies, not functions implemented by this
project.

| symbol | C type | provider |
|--------|--------|----------|
| `_ITM_deregisterTMCloneTable` | weak undefined | toolchain runtime |
| `_ITM_registerTMCloneTable` | weak undefined | toolchain runtime |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | libc |
| `__gmon_start__` | weak undefined | toolchain runtime |
| `__isoc99_scanf@GLIBC_2.7` | undefined | libc |
| `div@GLIBC_2.2.5` | undefined | libc |
| `printf@GLIBC_2.2.5` | undefined | libc |

Defined-symbol parity command:

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
```

Result: **empty (0 missing symbols)**.

- [x] Every C-defined dynamic symbol is exported by the Rust shared library.
- [x] Every C dynamic symbol name is present in the Rust dynamic symbol table.
