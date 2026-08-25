# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/debug/libdriver.so
```

## C-defined public symbols

| symbol | C `.so` | Rust `.so` |
|--------|----------|------------|
| `decode_base64` | `T` | `T` |

The C library has no other defined dynamic symbols. The undefined entries from
`nm -D` are libc/toolchain imports (`calloc`, `free`, `malloc`, `strlen`, and
weak ELF runtime hooks), not library API exports.

- [x] Missing C-defined symbols in Rust: 0
- [x] Undefined non-libc API symbols in Rust: 0
