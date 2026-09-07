# Dynamic Symbol Surface

Generated from the built shared libraries with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-QXNeUh.so
nm -D --defined-only target/release/libsynth_pair_lib.so
```

| C symbol | C type | Rust symbol present | Notes |
|----------|--------|---------------------|-------|
| `synth_pair` | `T` | yes | Exported as `extern "C"` with an unmangled name. |

Missing C symbols in Rust: **0**

The C library has no other defined dynamic symbols.

Final verification:

- [x] Exact `nm -D --defined-only` symbol diff is empty.
- [x] `ldd -r` reports no unresolved symbols in either shared library.
- [x] The result holds after both default and `--no-default-features` builds.
