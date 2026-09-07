# Dynamic symbol surface

Source library: `../c_src/build/libdriver.so`

Mechanical inventory command:

```text
nm -D --format=posix ../c_src/build/libdriver.so
```

| symbol | C binding/type | C status | Rust status | parity |
|---|---:|---|---|:---:|
| `_ITM_deregisterTMCloneTable` | weak | undefined toolchain runtime import | undefined weak import | yes |
| `_ITM_registerTMCloneTable` | weak | undefined toolchain runtime import | undefined weak import | yes |
| `__cxa_finalize@GLIBC_2.2.5` | weak | undefined libc runtime import | undefined weak import | yes |
| `__gmon_start__` | weak | undefined toolchain runtime import | undefined weak import | yes |
| `driver` | global function | **defined public API** | **defined public API** | yes |
| `puts@GLIBC_2.2.5` | global function | undefined libc import | undefined libc import | yes |

The C library has exactly one defined dynamic symbol: `driver`. The Rust
library exports it with the exact same name. Rust has additional undefined
libc, pthread, unwinding, and compiler-runtime imports from the Rust runtime;
none is a missing C API implementation.

## Completion

- [x] Final release-build symbol diff has zero missing defined C symbols.
- [x] Rust has zero undefined non-libc/non-toolchain API symbols.
