# Dynamic Symbol Surface

Source artifact:
`c_src/build/libdriver_c.so`, built from the unmodified
`c_src/src/main.c` with:

```text
cc -shared -fPIC -o c_src/build/libdriver_c.so c_src/src/main.c
```

The checked-in CMake configuration only defines the `driver` executable, so it
does not itself emit a shared object. The command above compiles the same source
and default preprocessor configuration as a position-independent shared object.

## C-defined public symbols

Mechanically collected with
`nm -D --defined-only c_src/build/libdriver_c.so`.

| symbol | kind | Rust status before Phase A fixes |
|--------|------|----------------------------------|
| `driver` | `T` | missing: Cargo defined only a binary and the Rust function was private |
| `main` | `T` | missing: Cargo defined only a binary and had no Rust shared object |

## Imported runtime symbols

These are undefined libc/toolchain imports in the C shared object, not symbols
defined by this library:

| symbol |
|--------|
| `_ITM_deregisterTMCloneTable` |
| `_ITM_registerTMCloneTable` |
| `__ctype_b_loc@GLIBC_2.3` |
| `__cxa_finalize@GLIBC_2.2.5` |
| `__gmon_start__` |
| `getchar@GLIBC_2.2.5` |
| `printf@GLIBC_2.2.5` |
| `setlocale@GLIBC_2.2.5` |
| `tolower@GLIBC_2.2.5` |
| `toupper@GLIBC_2.2.5` |

## Completion

- [x] Rust defines `driver`.
- [x] Rust defines `main`.
- [x] C-defined-to-Rust-defined symbol diff is empty.
- [x] Rust has no undefined non-runtime symbol.
