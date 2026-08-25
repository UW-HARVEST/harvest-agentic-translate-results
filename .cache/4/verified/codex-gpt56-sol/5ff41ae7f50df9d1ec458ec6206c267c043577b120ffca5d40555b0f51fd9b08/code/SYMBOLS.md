# Dynamic Symbol Surface

Source artifact:
`c_src/build/libdriver_c.so`, built from the unchanged
`c_src/src/main.c` with `cc -shared -fPIC`.

The CMake project itself defines an executable, not a shared-library target.
The shared object above is therefore a verification artifact compiled from the
same source after the prescribed CMake build completed.

## C-defined public symbols

| symbol | C `nm -D` | Rust `nm -D` | status |
|--------|-----------:|--------------:|--------|
| `main` | `T` | `T` | [x] |

`main` is exported by the Rust `cdylib` through the `extern "C"` wrapper in
`src/lib.rs`.

## C shared-object dependencies

These are present in the complete `nm -D` output but are not symbols defined by
the C library. They are supplied by libc or ELF toolchain support and are not
part of the translated API.

| symbol | kind |
|--------|------|
| `_ITM_deregisterTMCloneTable` | weak undefined toolchain symbol |
| `_ITM_registerTMCloneTable` | weak undefined toolchain symbol |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined libc symbol |
| `__gmon_start__` | weak undefined toolchain symbol |
| `fgets@GLIBC_2.2.5` | undefined libc function |
| `fputs@GLIBC_2.2.5` | undefined libc function |
| `stdin@GLIBC_2.2.5` | undefined libc object |
| `stdout@GLIBC_2.2.5` | undefined libc object |

Completion criterion: `nm -D --defined-only` must report no C-defined symbol
missing from the Rust shared object. The final sorted symbol sets are both
exactly `main`; their difference is empty. `ldd -r` reports no unresolved
relocations.
