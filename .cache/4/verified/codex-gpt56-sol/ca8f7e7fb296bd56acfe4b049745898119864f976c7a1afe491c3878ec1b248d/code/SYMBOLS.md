# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

The C shared library has two globally defined dynamic symbols. The Rust
shared library exports both with the exact C name.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `FIO_createFilename_fromOutDir` | `T` | `FIO_createFilename_fromOutDir` | [x] |
| `extractFilename` | `T` | `extractFilename` | [x] |

Verification command:

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

Result: empty (zero missing symbols).

The undefined entries in the C library are libc/runtime imports. The Rust
library likewise has no undefined reference to either library-owned symbol.
`ldd -r` reports no unresolved relocation in either shared library.

- [x] Zero missing C exports in Rust
- [x] Zero unresolved shared-library relocations
