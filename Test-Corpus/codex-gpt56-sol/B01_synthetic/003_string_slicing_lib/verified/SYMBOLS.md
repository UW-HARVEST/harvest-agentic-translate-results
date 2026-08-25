# Dynamic Symbol Surface

Generated from the default C build:

```text
$ nm -D --defined-only c_src/build/libString_Slice.so
0000000000001129 T slice
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `slice` | Global function (`T`) | `slice` | Present [x] |

The C library's undefined dynamic symbols are the libc functions `printf`,
`puts`, and `strlen`, plus weak ELF runtime symbols. They are external
dependencies rather than library API. The Rust library has no unresolved
project symbols, and `ldd -r` resolves all of its runtime dependencies.

Feature matrix: `Cargo.toml` has no `[features]` section and CMake defines no
options, so the only valid configuration is `--no-default-features`.

Completion gate: [x] `nm -D` reports no C export missing from Rust.
