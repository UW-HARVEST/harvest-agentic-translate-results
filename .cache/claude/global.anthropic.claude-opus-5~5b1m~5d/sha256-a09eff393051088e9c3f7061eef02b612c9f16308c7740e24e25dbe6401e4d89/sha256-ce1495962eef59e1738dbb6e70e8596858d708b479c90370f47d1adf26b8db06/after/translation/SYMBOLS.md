# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` dynamic symbol table (`nm -D --defined-only c_src/build/libdriver.so`)

```
0000000000001173 T driver
```

That is the complete list: **1** exported symbol.

## Rust `.so` dynamic symbol table (`nm -D --defined-only translation/target/release/libdriver.so`, local/weak filtered)

```
0000000000011720 T driver
```

## Parity table

| # | C symbol | type | exported by Rust `.so`? | notes |
|---|----------|------|-------------------------|-------|
| 1 | `driver` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub extern "C" fn driver(x: f32)` | signature `void driver(float)` |

**Missing symbols: 0.**
**Extra non-libc/non-runtime symbols in Rust: 0.**

## Non-exported (file-local) C symbols — intentionally NOT exported

`nm` on the C object shows these `t`/`d`/`b` local symbols. They are `static`
or CRT/toolchain artifacts and are *not* part of the ABI surface, so the Rust
`.so` is correct not to export them:

| C local symbol | class | reason not exported |
|----------------|-------|---------------------|
| `print_hex` | `t` | declared `static` in `src/driver.c` |
| `_init`, `_fini` | `t` | linker-generated |
| `frame_dummy`, `__do_global_dtors_aux`, `register_tm_clones`, `deregister_tm_clones` | `t` | glibc CRT glue |
| `_DYNAMIC`, `_GLOBAL_OFFSET_TABLE_`, `__TMC_END__`, `__dso_handle`, `__frame_dummy_init_array_entry`, `__do_global_dtors_aux_fini_array_entry`, `completed.0` | `d`/`b` | linker/CRT data |

`print_hex` IS translated in Rust (as a private `unsafe fn print_hex`), matching
the C's `static` linkage — the implementation is present, only the (correctly
absent) export is missing. No stubs, no `unimplemented!()` anywhere.

## Translated-source completeness

| C source file | translated? | where |
|---------------|-------------|-------|
| `c_src/src/driver.c` (`print_hex`, `driver`) | YES, fully | `translation/src/lib.rs` |
| `c_src/include/driver.h` (`void driver(float)`) | YES | `translation/src/lib.rs` |

No C source file or function in `c_src/` is untranslated.

## Undefined (imported) symbols

The C `.so` imports exactly:

```
printf@GLIBC_2.2.5   U
putchar@GLIBC_2.2.5  U     <- GCC rewrites printf("\n") into putchar('\n')
_ITM_*/__cxa_finalize/__gmon_start__   w  (weak CRT hooks)
```

The Rust `.so` imports the same `printf` **and** the same `putchar` (LLVM
applies the identical `printf("\n")` → `putchar('\n')` simplification), because
the translation calls libc `printf` via `extern "C"` rather than using Rust's
`std::io`. Stdout formatting *and* stream buffering therefore go through the
identical libc code path in both libraries. The rest of the Rust `.so`'s
undefined list is std/`libunwind`/libc (`malloc`, `memcpy`, `_Unwind_*`, …).

**0 missing/undefined non-libc symbols in the Rust `.so`** — in particular no
unresolved `driver`/`print_hex`, i.e. no untranslated C module is being
referenced. Asserted by `tests/phase_d_symbols.rs::rust_so_has_no_unresolved_project_symbols`.

## Verdict

Symbol parity: **COMPLETE** (diff is empty).
