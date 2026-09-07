# SYMBOLS.md — Public symbol parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-0ThxNC.so`
- Rust `.so`: `translation/target/release/librgb_to_hsv_lib.so`

## C exported symbols (`nm -D --defined-only`, text/data only)

| # | symbol | type | exported by Rust `.so`? |
|---|--------|------|--------------------------|
| 1 | `rgb_to_hsv` | `T` (global text) | YES — `T rgb_to_hsv` |

## Result

```
$ nm -D --defined-only c_src/build/libharvest-work-0ThxNC.so
00000000000010f9 T rgb_to_hsv

$ nm -D --defined-only translation/target/release/librgb_to_hsv_lib.so
00000000000116a0 T rgb_to_hsv
```

Symbol diff (C-exported symbols missing from Rust): **EMPTY**.

The C library declares exactly one public entry point in
`c_src/include/lib.h` (`void rgb_to_hsv(float *dest, const float *src);`)
and `c_src/src/lib.c` contains no other functions, no file-static helpers with
external linkage, no macro-generated symbols, and no global data. There is no
untranslated C module: `src/lib.c` is the only source file listed in
`c_src/CMakeLists.txt`, and it is fully translated in `translation/src/lib.rs`.

Automated in `run_tests.sh`, which fails the run unless
`diff <(nm -D --defined-only <c.so>) <(nm -D --defined-only <rust.so>)` on the
symbol-name column is empty, for every profile and feature combination.

Undefined non-libc symbols in the Rust `.so`: none (verified with
`nm -D -u`; only the Rust/libc runtime imports appear, no unresolved
library-level references).
