# SYMBOLS.md — dynamic symbol parity

Derived mechanically from:

```sh
nm -D --defined-only  c_src/build/libharvest-work-XODnvq.so
nm -D --defined-only  translation/target/release/libhex2bin_lib.so
nm -D --undefined-only <both>
```

C translation unit set (from `c_src/CMakeLists.txt`): `src/lib.c` only.
Public header set: `include/lib.h` only.

## Defined (exported) symbols

| # | C symbol (`nm -D`) | type | Rust `.so` exports it? | notes |
|---|--------------------|------|------------------------|-------|
| 1 | `hex2bin`          | `T`  | YES (`T hex2bin`)      | `#[unsafe(no_mangle)] pub unsafe extern "C" fn hex2bin` in `src/lib.rs` |

**Missing from Rust `.so`: 0.** No macro-generated symbols exist in this C
source (no `#define`-generated function names, no aliases, no `__attribute__
((alias))`), so the exported surface is exactly one function.

Rust-only extra defined symbols: none in the text/global-data segment beyond
`hex2bin` (the remaining `nm -D` entries in the Rust `.so` are read-only/`.bss`
runtime objects belonging to the Rust std panic/backtrace machinery, not API).

## Undefined (imported) symbols

| library | non-libc undefined symbols |
|---------|----------------------------|
| C `.so` | none (`strchr`, `__cxa_finalize`, `__gmon_start__`, `_ITM_*` are libc/toolchain) |
| Rust `.so` | none (`_Unwind_*` from libgcc, everything else glibc: `malloc`, `memcpy`, `strlen`, `open64`, `mmap64`, …) |

`strchr` is used by the C; the Rust reimplements it inline as
`c_strchr_found` (including the C quirk that the terminating NUL is part of the
searched string), so `strchr` does not need to be imported.

## Completion gate

- [x] `nm -D` shows **0 missing** exported symbols in the Rust `.so`.
- [x] `nm -D` shows **0 undefined non-libc** symbols in the Rust `.so`.
- [x] No whole C module/file was skipped by the translation (`lib.c` is the only
      source file, and its only function is translated).
