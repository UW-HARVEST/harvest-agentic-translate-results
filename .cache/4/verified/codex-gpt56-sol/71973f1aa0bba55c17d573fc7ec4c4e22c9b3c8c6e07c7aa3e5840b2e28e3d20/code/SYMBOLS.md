# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Command: `nm -D --defined-only c_src/build/libtranslated_rust.so`

| symbol | C type | Rust status |
|--------|--------|-------------|
| `float2half` | `T` | exported as `float2half` |

The unfiltered C `nm -D` output also contains the undefined weak runtime
symbols `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`. They are toolchain runtime
imports, not symbols implemented by this library.

Missing defined C symbols in Rust: **0**

