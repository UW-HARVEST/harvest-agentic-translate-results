# Exported Symbol Surface

Source binary: `../c_src/build/libdriver.so`

Command used:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust symbol present | Rust implementation |
|----------|--------|---------------------|---------------------|
| `driver` | `T` | yes | `src/lib.rs` |
| `foo` | `T` | yes | `src/lib.rs` |

The C shared library exports no other defined dynamic symbols. The corresponding
command on `target/release/libdriver.so` reports both exact names.

Completion:

- [x] Every defined dynamic C symbol is exported by Rust.
- [x] Missing-symbol diff is empty.
- [x] Rust has no undefined non-libc project symbols.
