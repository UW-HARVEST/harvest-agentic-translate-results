# SYMBOLS.md — Symbol parity between C `.so` and Rust `.so`

Generated mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/debug/libdriver.so
```

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** → the only valid feature set is the
  empty one. `cargo check/build/test --no-default-features` is the single
  configuration (identical to the default configuration).
* `c_src/CMakeLists.txt` has **no `option()`, no `add_definitions`, no
  `target_compile_definitions`, no `#ifdef`** anywhere in `c_src/` → the C side
  likewise has exactly one build configuration.

Verified: `grep -rn "cfg(feature" src/` → no matches; `grep -n "option\|ifdef" c_src/CMakeLists.txt` → no matches.

## Exported (defined, dynamic) symbols

| # | C `.so` symbol | type | Rust `.so` exports it? | notes |
|---|----------------|------|------------------------|-------|
| 1 | `UTIL_createLinePointers` | `T` (global text) | **YES** (`T`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |

**C-exported symbols missing from the Rust `.so`: 0.**

The C library consists of a single translation unit (`c_src/src/lib.c`, 34 lines)
declaring a single public function in `c_src/include/lib.h`. Nothing was skipped
by the translation: the whole C source is covered by `src/lib.rs`. No stubs, no
`unimplemented!()`.

## Undefined (imported) symbols

C `.so` imports: `malloc@GLIBC_2.2.5`, `free@GLIBC_2.2.5`, plus the standard weak
ELF/CRT symbols (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`).

Rust `.so` imports the same `malloc`/`free` (the translation deliberately calls
the **C allocator**, so a caller may `free()` the returned block exactly as with
the C library) plus the usual Rust-runtime libc/unwind imports
(`_Unwind_*`, `memcpy`, `memset`, `open64`, `mmap64`, `pthread_key_*`, `abort`,
…). All of these are provided by glibc / libgcc.

**Undefined non-libc / non-unwind symbols in the Rust `.so`: 0.**

Confirmed loadable with no missing dependencies:

```
ldd -r target/debug/libdriver.so   # no "undefined symbol" lines
ldd -r c_src/build/libdriver.so    # no "undefined symbol" lines
```

## Rust-internal symbols

The Rust `.so` additionally exports mangled Rust symbols (`_ZN…` / `_R…`) from
`core`/`std` monomorphisations. These are *extra* symbols, not missing ones, and
are irrelevant to C ABI parity.

## Result

`nm -D` symbol diff (C-defined minus Rust-defined) is **EMPTY**. ✔
