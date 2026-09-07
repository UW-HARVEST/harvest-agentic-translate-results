# Dynamic Symbol Surface

Source library: `../c_src/build/libpow.so`

Command used to derive the public API:

```text
nm -D --defined-only ../c_src/build/libpow.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `my_pow` | `T` (global text/function) | `my_pow` | [x] present |

No C-defined public symbol is missing from `target/release/libpow.so`.

The complete unfiltered `nm -D` output also contains undefined imports
(`__errno_location`, `fprintf`, `pow`, and `stderr`) and weak toolchain/runtime
imports. These are dependencies of the shared object, not public symbols
defined by the C library.
