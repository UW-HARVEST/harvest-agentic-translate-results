# Dynamic Symbol Surface

Reference library: `c_src/build/libtranslated_rust.so`

Extraction command:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C address | type | symbol | Rust export |
|-----------|------|--------|-------------|
| `0000000000001670` | `T` | `tritanopia` | [x] |

The unfiltered `nm -D` output also contains the undefined runtime symbols
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, and `pow`. These are shared-library/runtime dependencies, not
symbols defined by the C library.

Completion:

- [x] Every C-defined dynamic symbol is defined by the Rust shared library.
- [x] Rust has no undefined non-runtime/non-libc symbols.
