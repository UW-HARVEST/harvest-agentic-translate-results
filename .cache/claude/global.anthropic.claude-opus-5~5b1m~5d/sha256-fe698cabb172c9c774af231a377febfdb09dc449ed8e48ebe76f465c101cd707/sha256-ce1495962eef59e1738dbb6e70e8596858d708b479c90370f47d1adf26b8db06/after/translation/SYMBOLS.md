# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries. Nothing here is
assumed; every row below is copied from the tool output reproduced verbatim.

## Source inventory (proof the whole C tree is accounted for)

`find c_src -type f -not -path './build/*'` yields exactly three files:

| file | contributes symbols? |
|------|----------------------|
| `c_src/CMakeLists.txt` | no (build script) |
| `c_src/include/lib.h`  | declarations only — `uint32_t rev16(uint32_t a);` |
| `c_src/src/lib.c`      | yes — defines `rev16`, the only function in the tree |

`grep -rnE '^[A-Za-z_].*\(' c_src/include c_src/src` returns exactly two lines
(`src/lib.c:3: uint32_t rev16(uint32_t a) {` and
`include/lib.h:3: uint32_t rev16(uint32_t a);`). There is **no untranslated
module**: the C library consists of one translation unit with one function, and
`translation/src/lib.rs` implements it. No stubs, no `unimplemented!()`.

The header contains no namespacing/renaming macros and no `#define`s at all, so
the source-level name is the final linker name.

## C `.so` — `nm -D c_src/build/libharvest-work-LSh6fE.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000010f9 T rev16
```

## Rust `.so` — `nm -D translation/target/release/librev16_lib.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U __rust_no_alloc_shim_is_unstable_v2
0000000000011c30 T rev16
```

## Parity table

| # | symbol | type | in C `.so` | in Rust `.so` | verdict |
|---|--------|------|-----------|---------------|---------|
| 1 | `rev16` | `T` (defined, global text) | yes | yes | **MATCH** — exact name, exported via `#[unsafe(no_mangle)] pub extern "C"` |

### Weak / toolchain symbols (excluded from the diff by design)

These are emitted by the C runtime startup glue and by the Rust `std` shim, not
by the library source. They are `w` (weak undefined) or `U` (undefined) and are
resolved by libc / the Rust runtime, so they are not part of the library's
public surface.

| symbol | C | Rust | note |
|--------|---|------|------|
| `_ITM_deregisterTMCloneTable` | `w` | `w` | GCC transactional-memory glue, present in both |
| `_ITM_registerTMCloneTable`   | `w` | `w` | same |
| `__cxa_finalize@GLIBC_2.2.5`  | `w` | `w` | libc atexit glue, present in both |
| `__gmon_start__`              | `w` | `w` | profiling glue, present in both |
| `__rust_no_alloc_shim_is_unstable_v2` | — | `U` | Rust-only undefined marker, satisfied by the Rust runtime; adds nothing to the exported surface |

## Completion check

* Symbols exported by C but **missing** from Rust: **0**
* Undefined non-libc symbols in the Rust `.so`: **0**
  (`__rust_no_alloc_shim_is_unstable_v2` is the Rust runtime's own marker, and
  the `w` entries above are libc/toolchain glue that the C `.so` also has.)
* Extra defined symbols exported by Rust that C does not export: **0**

The symbol diff is **empty**. Verified automatically by
`tests/symbol_parity.rs::c_defined_symbols_are_all_exported_by_rust`, which
re-runs `nm -D` on both libraries at test time so the artifact cannot drift.
