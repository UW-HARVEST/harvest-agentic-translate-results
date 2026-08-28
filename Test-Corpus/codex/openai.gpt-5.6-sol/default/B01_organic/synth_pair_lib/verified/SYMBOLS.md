# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-jskHrr.so`

Inventory command:

```sh
nm -D ../c_src/build/libharvest-work-jskHrr.so
```

## C-defined public symbols

| symbol | C `nm -D` type | Rust export | status |
|--------|----------------|-------------|--------|
| `synth_pair` | `T` | `T` | [x] |

## Toolchain weak imports

These are undefined weak runtime references, not symbols defined by the C
library. They are listed because they appear in the unfiltered `nm -D` output.

| symbol | C `nm -D` type | present in Rust `nm -D` |
|--------|----------------|--------------------------|
| `_ITM_deregisterTMCloneTable` | `w` | yes |
| `_ITM_registerTMCloneTable` | `w` | yes |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | yes |
| `__gmon_start__` | `w` | yes |

## Completion

- [x] C-defined symbols missing from the Rust shared library: 0
- [x] C-defined symbols left undefined by the Rust shared library: 0
