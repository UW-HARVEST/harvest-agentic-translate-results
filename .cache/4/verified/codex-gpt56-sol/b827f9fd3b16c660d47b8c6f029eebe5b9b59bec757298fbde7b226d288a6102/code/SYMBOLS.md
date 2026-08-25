# Dynamic Symbol Surface

Derived with:

```text
nm -D --defined-only --format=posix c_src/build/libtranslated_rust.so
```

| # | C symbol | Type | C source | Rust export | Status |
|---|----------|------|----------|-------------|--------|
| 1 | `tfm` | `T` | `c_src/src/lib.c:5` | `src/lib.rs:41` | [x] |

The public header declares the same single function:

```c
void tfm(float *dest, const float *src, int count);
```

`comm` over the defined global dynamic-symbol names reports no C symbols
missing from `target/release/libtfm_lib.so` and no extra Rust symbols.

The C library's only required function dependency is `sqrtf` from `libm`.
The Rust library also resolves `sqrtf` from `libm`; its remaining undefined
symbols are standard `libc`/`libgcc_s` runtime dependencies.
