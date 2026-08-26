# Dynamic Symbol Surface

Derived from:

```text
nm -D c_src/build/libStaticLoop.so
```

## Defined public API symbols

| C symbol | C type | Rust symbol present |
|----------|--------|---------------------|
| `driver` | `T` | yes |
| `static_sum` | `T` | yes |

## Imported and toolchain symbols

These are not library API implementations and do not require Rust exports.

| Symbol | C type | Classification |
|--------|--------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | toolchain weak import |
| `_ITM_registerTMCloneTable` | `w` | toolchain weak import |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc weak import |
| `__gmon_start__` | `w` | toolchain weak import |
| `printf@GLIBC_2.2.5` | `U` | libc import |

Missing C API symbols in Rust: **0**

Status: **complete** (`nm -D --defined-only` symbol diff is empty).
