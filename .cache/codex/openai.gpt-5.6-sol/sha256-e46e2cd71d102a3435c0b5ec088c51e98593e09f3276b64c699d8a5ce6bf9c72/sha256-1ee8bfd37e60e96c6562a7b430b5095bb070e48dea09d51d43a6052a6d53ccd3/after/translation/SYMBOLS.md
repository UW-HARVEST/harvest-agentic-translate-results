# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-YACHJq.so
nm -D --defined-only target/release/libbetagamma_lib.so
```

Only defined public symbols are API exports. Undefined `malloc`, `calloc`,
`free`, and `strcpy` entries in the C library are libc dependencies, not
library exports.

| C symbol | C type | Rust type | Rust export |
|----------|--------|-----------|-------------|
| `allocate_block` | `T` | `T` | present |
| `betagamma` | `T` | `T` | present |
| `compute_hash` | `T` | `T` | present |
| `create_block` | `T` | `T` | present |
| `free_block` | `T` | `T` | present |

- [x] Missing C exports in Rust: **0**
- [x] Undefined non-libc C symbols missing from Rust: **0**
