# Dynamic Symbol Surface

Derived from:

```text
nm -D c_src/build/libtranslated_rust.so
nm -D --defined-only --extern-only --format=posix c_src/build/libtranslated_rust.so
```

## Public C exports

| symbol | C type | Rust export | status |
|---|---:|---|---|
| `bitwriter_add` | `T` | `bitwriter_add` | [x] |

The C shared object has one defined external dynamic symbol. The C-to-Rust
defined-symbol difference is empty.

## Undefined weak runtime symbols

These entries also appear in the unfiltered `nm -D` output. They are toolchain
runtime imports, not library API exports.

| symbol | type |
|---|---:|
| `_ITM_deregisterTMCloneTable` | `w` |
| `_ITM_registerTMCloneTable` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` |
| `__gmon_start__` | `w` |

