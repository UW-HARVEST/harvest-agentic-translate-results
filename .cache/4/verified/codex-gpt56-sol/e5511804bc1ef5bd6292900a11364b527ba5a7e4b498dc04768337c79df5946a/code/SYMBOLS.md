# Dynamic Symbol Surface

Reference library: `c_src/build/libdriver.so`

Extraction command:

```sh
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | C type | Rust implementation/export | `nm -D` parity |
|----------|--------|----------------------------|-----------------|
| `driver` | `T` | `src/lib.rs::driver` (`extern "C"`, `no_mangle`) | [x] |
| `fma_array` | `T` | `src/lib.rs::fma_array` (`extern "C"`, `no_mangle`) | [x] |

Undefined C-library symbols (`memcpy`, `printf`) and weak toolchain/runtime
symbols are imports, not public definitions, and are therefore not parity
requirements.

Missing Rust definitions: 0.

Unresolved runtime symbols reported by `ldd -r`: 0.
