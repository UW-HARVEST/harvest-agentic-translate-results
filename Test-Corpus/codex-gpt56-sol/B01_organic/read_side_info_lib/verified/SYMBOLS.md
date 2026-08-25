# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `read_side_info` | `T` | `read_side_info` | [x] |

The C shared object has no other defined dynamic symbols. Its only undefined
symbols are weak toolchain/runtime symbols (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, and
`__gmon_start__`); none is a library API dependency.

Completion check: **0 C symbols missing from the Rust shared object**.
