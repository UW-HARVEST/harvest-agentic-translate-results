# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` defined dynamic symbols (non-libc / non-toolchain)

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `UTIL_createLinePointers` | `T` (text, global) | YES — `T`, exact name |

`c_src/include/lib.h` declares exactly one public entry point, and
`c_src/src/lib.c` defines exactly one function. There is no second translation
unit, no macro-generated symbol family, and no static/inline helper that leaks a
symbol. So the complete C surface is one symbol.

## Rust `.so` extra defined symbols

Only the standard toolchain-emitted entries (`_init`, `_fini`,
`__rust_no_alloc_shim_is_unstable_v2`, `rust_eh_personality`, etc.). No missing
implementation, no stub, no `unimplemented!()`.

## Undefined (imported) symbols in the Rust `.so`

```sh
nm -D --undefined-only translation/target/release/libdriver.so
```

All undefined symbols are libc (`malloc`, `free`, and glibc/`ld` bookkeeping
such as `__libc_start_main`-family / `__cxa_*` stubs). **0 missing or undefined
non-libc symbols.**

## Verdict

Symbol diff (C-defined minus Rust-defined) is **EMPTY**. No module of the C
source was skipped: `src/lib.c` is the only source file listed in
`c_src/CMakeLists.txt` and it is fully translated in `translation/src/lib.rs`.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
