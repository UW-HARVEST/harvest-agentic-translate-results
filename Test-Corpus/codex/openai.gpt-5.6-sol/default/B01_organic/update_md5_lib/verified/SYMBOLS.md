# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-wBzseN.so
nm -D --defined-only target/release/libupdate_md5_lib.so
```

| # | C symbol | C type | Rust symbol | Rust type | Status |
|---|----------|--------|-------------|-----------|--------|
| 1 | `tflac_md5_addsample` | `T` | `tflac_md5_addsample` | `T` | [x] |
| 2 | `tflac_pack_u64le` | `T` | `tflac_pack_u64le` | `T` | [x] |
| 3 | `update_md5` | `T` | `update_md5` | `T` | [x] |

Missing C-defined dynamic symbols in Rust: **0**.

The remaining entries from plain `nm -D` on the C library are weak/runtime
imports (`_ITM_*`, `__cxa_finalize`, and `__gmon_start__`), not symbols defined
or implemented by this library.
