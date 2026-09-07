# Dynamic symbol surface

Reference library:
`../c_src/build/libharvest-work-dDNg7n.so`

Rust library:
`target/release/libflip_horizontal_lib.so`

The mechanically extracted defined dynamic API (`nm -D --defined-only`) is:

| C symbol | C type | Rust symbol present | Notes |
|----------|--------|---------------------|-------|
| `flip_horizontal` | `T` | yes | Exact exported name |

The other C `nm -D` entries (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`) are
undefined weak runtime/linker hooks, not symbols defined by this library.

Missing C-defined symbols in Rust: **0**

- [x] Every C-defined dynamic symbol is exported by the Rust shared object.
- [x] No C-defined symbol is left undefined in the Rust shared object.
