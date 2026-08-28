# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  : `c_src/build/libharvest-work-lSH44j.so`  (built by `cmake`, `add_library(... SHARED src/lib.c)`)
* Rust: `translation/target/{release,debug}/libconfusion_lib.so` (`crate-type = ["cdylib"]`)

## Public (exported, `T`) symbols

| # | C symbol (`nm -D`) | C source | Rust `#[unsafe(no_mangle)] extern "C"` | exported by Rust `.so` |
|---|--------------------|----------|----------------------------------------|------------------------|
| 1 | `create_state`   | `lib.c:57`  | `create_state`   | YES |
| 2 | `destroy_state`  | `lib.c:90`  | `destroy_state`  | YES |
| 3 | `process_buffer` | `lib.c:99`  | `process_buffer` | YES |
| 4 | `update_flags`   | `lib.c:126` | `update_flags`   | YES |
| 5 | `confuse_types`  | `lib.c:143` | `confuse_types`  | YES |
| 6 | `confusion`      | `lib.c:177` | `confusion`      | YES |

Only `confusion` appears in the installed public header `include/lib.h`; the
other five are *not* declared in the header but **are** exported with default
visibility from the C `.so`, therefore they are part of the ABI surface and are
tested directly.

There are no macro-generated symbols: the only macros in `lib.c`
(`STRINGIFY`, `DEBUG_VAR`, `LOG_OPERATION`) expand to `printf` call sites, not
to definitions.

### Symbol diff

```
comm -23 <(C exported)  <(Rust exported)   -> (empty)   # nothing missing in Rust
```

Verified by `tests/phase_d_symbols.rs::symbol_parity_c_so_vs_rust_so`, which
runs `nm -D --defined-only` on both objects at test time and asserts the
difference is empty. Result: **0 missing symbols.**

## Undefined (imported) symbols

The C object imports `malloc`, `free`, `memchr`, `printf`, `puts`, `snprintf`,
`strlen` from `libc.so.6` (`puts` is GCC's optimisation of
`printf("literal\n")` — byte-identical output).

The Rust object imports the same seven glibc entry points (the translation
calls the *real* libc `printf`/`snprintf`/`malloc`/`free`/`strlen`/`memchr`
through `extern "C"`, which is what keeps `%f`/`%u`/`%d` formatting and the
allocator ABI identical), plus the ordinary Rust runtime imports
(`__errno_location`, `memcpy`, `mmap64`, `_Unwind_*`, …). All of those resolve
against `libc.so.6` / `libgcc_s.so.1`.

* Non-libc / non-libgcc undefined symbols in the Rust `.so`: **0**
* `ldd -r` reports no unresolved symbols for either object.
