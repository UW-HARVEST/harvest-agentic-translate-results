# Dynamic symbol surface

Generated from the built shared objects with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-6IJQh0.so
nm -D --defined-only target/release/libmemchra2_lib.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `memchra2` | `T` | `memchra2` (`T`) | [x] exact match |

The C shared object exports one defined dynamic symbol. The Rust shared object
exports the same one defined dynamic symbol, with the exact name.

`nm -D --undefined-only` reports only libc, libgcc/unwind, pthread, and ELF
runtime imports. `ldd -r` is used by the completion check to reject unresolved
non-system symbols.
