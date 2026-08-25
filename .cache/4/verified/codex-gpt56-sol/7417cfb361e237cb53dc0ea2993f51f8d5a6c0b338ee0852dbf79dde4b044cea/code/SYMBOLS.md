# Dynamic Symbol Surface

Source artifact:
`c_src/build/libstatic_alias_c.so`, built from `c_src/src/main.c` with
position-independent code.

Command:

```sh
nm -D --defined-only c_src/build/libstatic_alias_c.so
```

| C symbol | C type | Rust implementation at inventory time | Resolution |
|----------|--------|---------------------------------------|------------|
| `main` | `T` | Binary-only Rust `main`; no C ABI export | Added an `extern "C"` export preserving C argument parsing, output, and return values. |
| `static_alias` | `T` | Internal safe model; no C ABI export and no process-static storage | Added an `extern "C"` export with the same pointer aliasing and static-storage behavior. |

The C object also imports `printf`, `puts`, and `strtol` from libc. These are
runtime dependencies, not symbols implemented by this library.

Final parity is checked with:

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libstatic_alias_c.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/libstatic_alias.so | awk '{print $3}' | sort -u)
```

- [x] `main` exported by Rust
- [x] `static_alias` exported by Rust
- [x] Final missing-symbol diff is empty
