# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-rjuFwk.so
nm -D --defined-only target/release/libtfm_lib.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `tfm` | `T` | `tfm` | `T` | [x] exact export match |

The C library also has one undefined runtime dependency, `sqrtf@GLIBC_2.2.5`;
it is not a public symbol defined by the library and therefore is not an export
that the Rust library must reproduce.

Missing C exports in Rust: **0**.
