# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libdriver_c.so
```

The shared object is linked from CMake's PIC object for `src/main.c`. CMake
defines only an executable target, so this preserves the compiled translation
unit without changing `c_src/`.

## Defined Public API

| C address | type | symbol | Rust `target/release/libdriver.so` |
|-----------|------|--------|------------------------------------|
| `0000000000001198` | `T` | `main` | [x] exported as `main` |

`foo` is `static` in C and therefore is not part of the dynamic API.

## Imported Runtime Symbols

These are the remaining entries printed by `nm -D`; they are libc/toolchain
imports rather than library API.

| type | symbol |
|------|--------|
| `w` | `_ITM_deregisterTMCloneTable` |
| `w` | `_ITM_registerTMCloneTable` |
| `w` | `__cxa_finalize@GLIBC_2.2.5` |
| `w` | `__gmon_start__` |
| `U` | `__isoc99_scanf@GLIBC_2.7` |
| `U` | `puts@GLIBC_2.2.5` |

Defined-symbol parity command:

```text
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver_c.so | awk '{print $3}' | sort) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort)
```

Result: empty (zero C API symbols missing from Rust).
