# Dynamic Symbol Surface

Reference library:
`../c_src/build/libharvest-work-v0LDLj.so`

Rust library:
`target/release/libdequantize_granule_lib.so`

Mechanical inventory command:

```sh
nm -D --defined-only <library>
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `dequantize_granule` | `T` | `dequantize_granule` | [x] |

The C library has no other defined dynamic symbols. `get_bits` is `static` and
does not appear in the dynamic symbol table.

Missing C symbols in Rust: **0**

Undefined non-runtime symbols required from Rust: **0**. The Rust library's
undefined dynamic symbols are supplied by libc, libgcc unwinding, pthreads, or
the ELF runtime.
