# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The default CMake configuration has one shared-library target and no build
options or conditional source files.

| C address | type | symbol | Rust export |
|-----------|------|--------|-------------|
| `0x1533` | `T` | `confuse_types` | [x] |
| `0x16a4` | `T` | `confusion` | [x] |
| `0x11b9` | `T` | `create_state` | [x] |
| `0x12ea` | `T` | `destroy_state` | [x] |
| `0x1329` | `T` | `process_buffer` | [x] |
| `0x1403` | `T` | `update_flags` | [x] |

The C library's remaining dynamic symbols are weak runtime hooks or imported
libc functions: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `free`, `malloc`, `memchr`, `printf`,
`puts`, `snprintf`, and `strlen`. They are not library-defined API symbols.

Current defined-symbol difference (C minus Rust): empty.

- [x] All six C-defined dynamic symbols are exported by the Rust cdylib under
  the exact same names.
- [x] The Rust cdylib has no unresolved non-runtime library API symbols.
