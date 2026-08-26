# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only --format=posix c_src/build/libtranslated_rust.so
```

The C shared library has one public defined dynamic symbol:

| # | symbol | type | C source | Rust implementation/export | parity |
|---|--------|------|----------|----------------------------|--------|
| 1 | `premultiply` | `T` | `c_src/src/lib.c:3` | `src/lib.rs:26`, `extern "C"` with `no_mangle` | [x] |

The weak runtime symbols shown by unfiltered `nm -D` are undefined toolchain
imports, not public symbols implemented by this library.
