# Dynamic symbol surface

Source library:
`../c_src/build/libharvest-work-7NfxTl.so`

Rust library:
`target/release/libupdate_frame_header_lib.so`

The public API inventory is the set emitted by:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-7NfxTl.so
```

| Symbol | C type | Rust type | Status |
|---|---:|---:|---|
| `update_frame_header` | `T` | `T` | Present |

The unfiltered C `nm -D` output also contains the following undefined weak
toolchain imports. They are not library exports or API symbols:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`.

Missing defined C symbols in Rust: **0**

- [x] Final release-build symbol diff is empty.
- [x] Rust has no undefined non-system library symbols.
