# Dynamic Symbol Surface

Source binary: `c_src/build/libdriver.so`, built from the unmodified C source
with the command in the verification prompt.

Command used:

```text
nm -D c_src/build/libdriver.so
```

## Defined public API

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `driver` | `T` | `driver` | present |
| `print_foo` | `T` | `print_foo` | present |

`print_foo` is not declared in `include/driver.h`, but it has external linkage
in `src/driver.c` and is present in the dynamic symbol table, so it is part of
the mechanically observed surface.

## Undefined/weak runtime imports

These entries are emitted by the C compiler/linker and are not library API
implementations:

| Symbol | C type | Rust `.so` status |
|--------|--------|-------------------|
| `_ITM_deregisterTMCloneTable` | `w` | imported weakly |
| `_ITM_registerTMCloneTable` | `w` | imported weakly |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | imported weakly |
| `__gmon_start__` | `w` | imported weakly |
| `printf@GLIBC_2.2.5` | `U` | imported |

## Completion

- [x] Every C-defined dynamic symbol is defined by the Rust shared library.
- [x] Missing C-defined symbols: 0.
- [x] Undefined non-runtime/non-libc symbols in the Rust shared library: 0.
