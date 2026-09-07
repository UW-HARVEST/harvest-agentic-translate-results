# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only --format=posix ../c_src/build/libharvest-work-uEEezf.so
nm -D --defined-only --format=posix target/release/libbetagamma_lib.so
```

The C library exports five public function symbols. The C header declares only
`betagamma`, but all dynamically exported symbols are part of this verification
surface.

| C symbol | kind | Rust symbol | status |
|----------|------|-------------|--------|
| `allocate_block` | `T` | `allocate_block` | [x] present |
| `betagamma` | `T` | `betagamma` | [x] present |
| `compute_hash` | `T` | `compute_hash` | [x] present |
| `create_block` | `T` | `create_block` | [x] present |
| `free_block` | `T` | `free_block` | [x] present |

Missing C symbols in Rust: **0**.

Undefined non-libc C symbols in Rust: **0**.
