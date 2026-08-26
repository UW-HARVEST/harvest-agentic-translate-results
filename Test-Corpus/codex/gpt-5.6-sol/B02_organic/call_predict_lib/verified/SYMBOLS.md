# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

Toolchain-only ELF symbols are omitted. The complete public C API surface is:

| C symbol | C type | Rust `.so` status |
|----------|--------|--------------------|
| `call_predict` | `T` | [x] exported as `call_predict` |

`c_src/include/lib.h` declares `get_predict_func(int)`, but the C source does
not define that name and the built C shared object does not export it. It is
therefore not part of the callable C `.so` ABI used for differential testing.

Completion:

- [x] No C dynamic symbol is missing from the Rust shared object.
- [x] The Rust shared object has no undefined non-libc project symbols.
