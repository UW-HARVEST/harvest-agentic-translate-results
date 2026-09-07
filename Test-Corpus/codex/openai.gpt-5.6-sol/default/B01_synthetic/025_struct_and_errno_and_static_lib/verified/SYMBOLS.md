# Dynamic symbol surface

Derived from:

```text
nm -D c_src/build/libdriver.so
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Defined public symbols

| C symbol | C type | Rust symbol present | Source |
|----------|--------|---------------------|--------|
| `driver` | `T` | [x] | Declared in `include/driver.h`; defined in `src/driver.c` |
| `run` | `T` | [x] | Defined with external linkage in `src/driver.c` |

## C shared-library imports

The remaining entries from `nm -D` are libc/toolchain imports, not symbols
implemented by this library:

```text
_ITM_deregisterTMCloneTable
_ITM_registerTMCloneTable
__cxa_finalize@GLIBC_2.2.5
__errno_location@GLIBC_2.2.5
__gmon_start__
printf@GLIBC_2.2.5
puts@GLIBC_2.2.5
strtol@GLIBC_2.2.5
```

## Phase D status

- [x] Every C-defined dynamic symbol is exported by the Rust shared object.
- [x] Missing C-defined symbols: 0.
- [x] Unresolved dynamic relocations reported by `ldd -r`: 0; runtime
  imports are satisfied by libc, libgcc, and weak toolchain symbols.
