# Dynamic symbol surface

Generated from:

```text
nm -D ../c_src/build/libharvest-work-Gc1ILW.so
nm -D target/release/libupdate_frame_header_lib.so
```

## Complete C `nm -D` surface

| C nm type | symbol | Rust nm status | classification |
|---|---|---|---|
| `w` | `_ITM_deregisterTMCloneTable` | present as undefined weak | toolchain runtime import |
| `w` | `_ITM_registerTMCloneTable` | present as undefined weak | toolchain runtime import |
| `w` | `__cxa_finalize@GLIBC_2.2.5` | present as undefined weak | libc runtime import |
| `w` | `__gmon_start__` | present as undefined weak | toolchain runtime import |
| `T` | `update_frame_header` | present as defined global (`T`) | public library export |

## Required defined public exports

| symbol | C | Rust | status |
|---|---|---|---|
| `update_frame_header` | defined global | defined global | [x] |

Missing C-defined public symbols in Rust: **0**.

