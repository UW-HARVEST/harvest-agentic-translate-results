# Dynamic symbol surface

Derived from:

```text
nm -D ../c_src/build/libharvest-work-xqsGFd.so
nm -D target/release/libcontrast_ratio_lib.so
```

## C-defined public API symbols

| symbol | C type | Rust type | Rust parity |
|---|---:|---:|---|
| `contrast_ratio` | `T` | `T` | [x] |

The C shared object defines no other global/weak dynamic symbols. Its complete
remaining `nm -D` surface consists of dynamic-linker dependencies:

| symbol | C type | Rust has compatible dynamic dependency |
|---|---:|---:|
| `_ITM_deregisterTMCloneTable` | `w` | yes |
| `_ITM_registerTMCloneTable` | `w` | yes |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | yes |
| `__gmon_start__` | `w` | yes |
| `pow@GLIBC_2.29` | `U` | yes |

Completion check: [x] zero C-defined public symbols are missing from Rust.
