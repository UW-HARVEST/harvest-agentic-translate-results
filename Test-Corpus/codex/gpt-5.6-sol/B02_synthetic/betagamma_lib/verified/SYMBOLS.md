# Dynamic Symbol Surface

Source: `nm -D` and `nm -D --defined-only` on
`c_src/build/libtranslated_rust.so`, built from the unmodified C source with
the default CMake configuration.

## Defined API Symbols

| C symbol | C type | Rust type | status |
|----------|--------|-----------|--------|
| `allocate_block` | `T` | `T` | [x] |
| `betagamma` | `T` | `T` | [x] |
| `compute_hash` | `T` | `T` | [x] |
| `create_block` | `T` | `T` | [x] |
| `free_block` | `T` | `T` | [x] |

## C Dynamic Dependencies

These are undefined imports, not API definitions that the Rust library must
export: `calloc@GLIBC_2.2.5`, `free@GLIBC_2.2.5`,
`malloc@GLIBC_2.2.5`, and `strcpy@GLIBC_2.2.5`.

The C library also has the normal weak toolchain imports
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`.

## Completion

- [x] The defined C symbol set minus the defined Rust symbol set is empty.
- [x] The Rust shared library has no undefined non-system library symbols.
