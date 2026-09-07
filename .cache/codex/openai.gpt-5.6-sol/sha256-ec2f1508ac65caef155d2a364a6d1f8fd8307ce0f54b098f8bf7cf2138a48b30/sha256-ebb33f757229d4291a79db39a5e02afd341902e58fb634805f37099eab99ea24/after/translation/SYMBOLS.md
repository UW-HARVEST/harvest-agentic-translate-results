# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-Bpf12g.so
nm -D --defined-only target/release/libmax_size_frame_lib.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `max_size_frame` | `T` | `max_size_frame` (`T`) | present |

Missing C symbols in Rust: **0**

Extra Rust exports: **0**

Unresolved application/library symbols at load time (`ldd -r`): **0**.
The Rust standard library's toolchain/system imports resolve through the
installed `libc` and `libgcc_s`.

- [x] Phase D symbol parity complete.
