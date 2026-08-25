# Dynamic Symbol Surface

Generated from the default C shared library with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C address | type | symbol | Rust export | status |
|-----------|------|--------|-------------|--------|
| `0000000000001109` | `T` | `gaussian_kernel` | `T gaussian_kernel` | [x] |

The C library has one public defined dynamic symbol. Comparison against
`nm -D --defined-only target/debug/libgaussian_kernel_lib.so` reports no
missing symbols.

- [x] Zero C public symbols are missing from the Rust shared library.
- [x] Zero undefined non-system symbols are required by the Rust shared library.
