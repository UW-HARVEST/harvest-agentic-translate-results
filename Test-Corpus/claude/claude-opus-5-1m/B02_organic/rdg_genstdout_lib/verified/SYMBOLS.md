# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

* C  `.so`: `c_src/build/libdriver.so`   (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/debug/libdriver.so` (`cargo build`, `crate-type = ["cdylib"]`)

## Exported (defined, dynamic) symbols of the C `.so`

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001189 T extractFilename
00000000000011c7 T FIO_createFilename_fromOutDir
```

## Exported (defined, dynamic) symbols of the Rust `.so`

```
$ nm -D --defined-only target/debug/libdriver.so
00000000000128b0 T FIO_createFilename_fromOutDir
0000000000012e40 T extractFilename
```

## Parity table

| # | C symbol | declared in header | exported by Rust `.so` | Rust item |
|---|----------|--------------------|------------------------|-----------|
| 1 | `extractFilename` | no (non-`static`, so still exported) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn extractFilename` |
| 2 | `FIO_createFilename_fromOutDir` | `c_src/include/lib.h` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn FIO_createFilename_fromOutDir` |

**Missing symbols: 0.** Nothing had to be re-translated or newly exported: every
non-`static` function in `c_src/src/lib.c` (`extractFilename`,
`FIO_createFilename_fromOutDir`) has a real, fully translated implementation in
`src/lib.rs` — there are no stubs and no `unimplemented!()`.

Verified automatically by `tests/symbols.rs::c_exports_are_a_subset_of_rust_exports`,
which shells out to `nm -D --defined-only` on both libraries and asserts the C
export set minus the Rust export set is empty.

## Undefined symbols (informational)

The C `.so` imports only libc: `__errno_location`, `calloc`, `exit`, `fprintf`,
`memcpy`, `stderr`, `strerror`, `strlen`, `strrchr`.

The Rust `.so` imports the same libc entry points that the translation uses
(`__errno_location`, `calloc`, `exit`, `fprintf`, `stderr`, `strerror`,
`memcpy`) plus the symbols pulled in by the Rust standard library / unwinder
(`_Unwind_*`, `malloc`, `free`, `mmap64`, `pthread_key_create`, …). There are
**0 missing / undefined non-libc symbols**; every undefined symbol resolves out
of `libc.so.6`, `libgcc_s.so.1` or `libpthread` on the target platform, which is
confirmed by the fact that `libloading::Library::new()` on the Rust `.so`
succeeds in every test.

Note that the C library allocates its result with libc `calloc`; the Rust
translation deliberately calls libc `calloc` through an `extern "C"` binding
rather than the Rust allocator, so a caller may `free()` the returned pointer
exactly as with the C library. The tests rely on this (they `free()` both
results).
