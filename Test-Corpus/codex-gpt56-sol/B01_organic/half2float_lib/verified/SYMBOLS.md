# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libtranslated_rust.so`.

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `half2float` | `T` (function) | `half2float` | [x] |

The C shared object has no other defined public dynamic symbols. The Rust
shared object exports every C symbol; the missing-symbol set is empty.
`ldd -r target/release/libhalf2float_lib.so` reports no unresolved symbols.
