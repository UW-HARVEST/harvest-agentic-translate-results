# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-ACXBgi.so
nm -D --defined-only target/release/libnormalize_lib.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `normalize` | `T` | `normalize` | `T` | [x] present |

The C library has one globally defined dynamic symbol. Its other dynamic
entries are undefined runtime imports (`memset`, `sqrtf`) or weak toolchain
symbols, not public symbols implemented by this library.

Final Phase D checks:

- [x] Missing C-defined symbols in Rust: **0** (empty `comm -23` diff).
- [x] `ldd -r` reports no unresolved relocations for either shared library.
- [x] No Cargo feature table exists; default and `--no-default-features`
  verification both pass.
- [x] Neither build defines a binary target, so no stdout comparison applies.
