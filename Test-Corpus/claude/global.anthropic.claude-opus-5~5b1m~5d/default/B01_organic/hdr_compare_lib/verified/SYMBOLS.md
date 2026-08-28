# SYMBOLS.md — Phase A: exported-symbol surface

Mechanically derived from `nm -D` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-AdtfYb.so

# Rust
cd translation && cargo build --release
# -> translation/target/release/libhdr_compare_lib.so
```

## `nm -D` on the C `.so`

```
$ nm -D c_src/build/libharvest-work-AdtfYb.so
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001190 T hdr_compare
```

`$ nm -D --defined-only` (strong, defined, i.e. the real ABI):

```
0000000000001190 T hdr_compare
```

## `nm -D --defined-only` on the Rust `.so`

```
0000000000011c30 T hdr_compare
```

(plus the usual Rust/`cdylib` runtime-internal weak/undefined libc imports, which are
not part of the C ABI surface.)

## Symbol table

| # | symbol | C `.so` | Rust `.so` | source of truth | status |
|---|--------|---------|------------|-----------------|--------|
| 1 | `hdr_compare` | `T` (global text) | `T` (global text) | `c_src/include/lib.h:3`, `c_src/src/lib.c:9` | **present in both** |

## Non-exported (internal) C functions

| # | symbol | linkage in C | why it is not in `nm -D` | Rust counterpart |
|---|--------|--------------|--------------------------|------------------|
| 1 | `hdr_valid` | `static int` (`c_src/src/lib.c:3`) | internal linkage — never in the dynamic symbol table | private `unsafe fn hdr_valid` in `src/lib.rs`; correctly **not** exported |

`hdr_valid` MUST NOT be exported by the Rust `.so`: exporting it would be a symbol-set
mismatch in the other direction. It is verified indirectly, through every `hdr_compare`
call, in Phases B and C.

## Weak / linker-generated symbols

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`
and `__gmon_start__` are `w` (weak) symbols emitted by the C toolchain's CRT glue, not by
`src/lib.c`. They are not part of the library's API and are excluded from the parity
requirement (the Rust `cdylib` CRT glue emits its own equivalent set).

## Diff

```
$ comm -23 <(c_so_defined_globals) <(rust_so_defined_globals)
<empty>
```

**Missing from Rust: none. No module of the C source was left untranslated**
(`c_src/src/lib.c` is the only translation unit in `c_src/CMakeLists.txt`, and both of
its functions — one exported, one `static` — are translated in `translation/src/lib.rs`).
No symbol is stubbed: `hdr_compare` is a full translation, exhaustively verified in
`VERIFICATION.md`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc_s / unwinder imports
(`malloc`, `memcpy`, `dl_iterate_phdr`, `_Unwind_*`, …), which come from the `std` runtime
glue that every Rust `cdylib` links. `ldd` resolves to `libgcc_s.so.1`, `libc.so.6` and the
loader only. **0 unresolved non-libc symbols.**

## Automated re-checks

| test | asserts |
|---|---|
| `tests/symbols.rs::symbol_parity_c_so_vs_rust_so` | the symbol diff is empty, and the C's ABI is exactly `{hdr_compare}` |
| `tests/symbols.rs::rust_so_has_no_unresolved_non_libc_symbols` | every `U` symbol is libc/libgcc/unwinder |
| `tests/symbols.rs::hdr_valid_stays_internal_in_both` | neither `.so` exports `hdr_valid` |
| `tests/symbols.rs::both_sos_expose_a_callable_hdr_compare` | the exported symbol is callable through `dlsym` in both |
| `tests/symbols.rs::rust_so_has_no_null_pointer_precondition_check` | no `rustc` null-deref instrumentation (see the divergence write-up in `VERIFICATION.md`) |
| `verify.sh` | re-runs the `nm -D` diff for every feature combination x profile |
