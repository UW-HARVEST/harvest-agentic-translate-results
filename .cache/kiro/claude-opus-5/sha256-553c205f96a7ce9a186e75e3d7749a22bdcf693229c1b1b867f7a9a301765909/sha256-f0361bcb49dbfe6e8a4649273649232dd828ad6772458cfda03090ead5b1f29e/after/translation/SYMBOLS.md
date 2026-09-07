# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only c_src/build/libharvest-work-4IfzS6.so
nm -D --defined-only translation/target/release/librev16_lib.so
nm -D --undefined-only translation/target/release/librev16_lib.so
```

## C `.so` exported (defined) symbols

| # | symbol | type | notes |
|---|--------|------|-------|
| 1 | `rev16` | `T` (global text) | the only public symbol; declared in `c_src/include/lib.h` as `uint32_t rev16(uint32_t a)` |

Weak/loader-provided entries reported by `nm -D` on the C `.so` that are NOT
part of the library's API surface (toolchain/glibc boilerplate, excluded from
parity requirements):
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`.

## Rust `.so` exported (defined) symbols

| # | symbol | type | Rust item |
|---|--------|------|-----------|
| 1 | `rev16` | `T` (global text) | `#[unsafe(no_mangle)] pub extern "C" fn rev16(a: c_uint) -> c_uint` in `src/lib.rs` |

## Parity diff

| direction | missing symbols |
|-----------|-----------------|
| in C `.so` but not Rust `.so` | **(none)** |
| in Rust `.so` but not C `.so` | **(none)** |

No symbol required a new `#[no_mangle]` wrapper, and no C translation unit was
skipped: `c_src` contains exactly one source file (`src/lib.c`, 9 lines) and one
header (`include/lib.h`, 3 lines), both fully translated. There are no
namespace/renaming macros in the header, so linker names equal source names.

## Undefined symbols in the Rust `.so`

All `U`/`w` entries are libc, libdl, pthread, or Itanium-ABI unwinder imports
pulled in by the Rust standard library (`malloc`, `memcpy`, `dl_iterate_phdr`,
`_Unwind_*`, …). **0 missing/undefined non-libc symbols.**

## Gate

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
- [x] Every C-exported symbol is exported by the Rust `.so` under the exact
      same name.
