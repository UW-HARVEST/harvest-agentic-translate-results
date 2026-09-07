# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-p9lAnV.so
nm -D --defined-only target/release/libgaussian_kernel_lib.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `gaussian_kernel` | `T` | `gaussian_kernel` | `T` | present |

The mechanically sorted defined-symbol diff is empty. The only undefined
calculation dependency in the C library is `expf@GLIBC_2.27`; the Rust library
also resolves `expf@GLIBC_2.27`. Other undefined Rust symbols are Rust runtime
or libc/libgcc dependencies, not C-library API symbols.

Completion status: [x] final release-build symbol diff rechecked after tests.
