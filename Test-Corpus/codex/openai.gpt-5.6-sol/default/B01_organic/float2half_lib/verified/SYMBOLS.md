# Dynamic Symbol Surface

Built libraries:

- C: `../c_src/build/libharvest-work-Rg2BZs.so`
- Rust: `target/release/libfloat2half_lib.so`

The public surface below is derived from `nm -D --defined-only` on the C
shared library. Runtime/toolchain symbols that are undefined imports are not
part of the library API.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `float2half` | `T` | `float2half` (`T`) | [x] exact match |

Missing C symbols in Rust: **0**.
Undefined non-libc C API symbols in Rust: **0**.
