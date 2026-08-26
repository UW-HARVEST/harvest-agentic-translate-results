# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libtranslated_rust.so
nm -D target/release/libupdate_md5_lib.so
```

The C library has three defined public symbols. All three are defined by the
Rust library with the same names.

| C type | symbol | C source | Rust status |
|--------|--------|----------|-------------|
| `T` | `tflac_pack_u64le` | `c_src/src/lib.c:5` | [x] exported |
| `T` | `tflac_md5_addsample` | `c_src/src/lib.c:16` | [x] exported |
| `T` | `update_md5` | `c_src/src/lib.c:33` | [x] exported |

For completeness, these are the undefined weak runtime symbols also printed by
`nm -D` for the C library. They are not library API definitions, and all four
also occur in the Rust library's dynamic symbol table.

| C type | symbol | Rust dynamic table |
|--------|--------|--------------------|
| `w` | `_ITM_deregisterTMCloneTable` | [x] present |
| `w` | `_ITM_registerTMCloneTable` | [x] present |
| `w` | `__cxa_finalize@GLIBC_2.2.5` | [x] present |
| `w` | `__gmon_start__` | [x] present |

Defined-symbol diff: **empty (0 missing symbols)**.

