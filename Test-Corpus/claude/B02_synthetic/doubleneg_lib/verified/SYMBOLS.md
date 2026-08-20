# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

Derived mechanically from:

```
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libdoubleneg_lib.so
```

## Build-time configuration surface

`Cargo.toml` has **no `[features]` section**, therefore the complete set of valid
feature combinations is exactly one:

| # | feature combination | cargo invocation |
|---|---------------------|------------------|
| 1 | *(empty — no features; identical to default)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

`c_src/CMakeLists.txt` has no `option()`, no `target_compile_definitions`, and no
`#ifdef` in `src/lib.c`, so the C side likewise has exactly one configuration
(one `SHARED` library, `src/lib.c`, linked against `m`).

## Exported (defined, dynamic) symbols

| # | C symbol (`nm -D` on C `.so`) | present in Rust `.so`? | Rust definition site |
|---|-------------------------------|------------------------|----------------------|
| 1 | `convert_double_to_int` | YES (`T`) | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn convert_double_to_int` |
| 2 | `find_value_in_buffer`  | YES (`T`) | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn find_value_in_buffer` |
| 3 | `process_negation`      | YES (`T`) | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn process_negation` |
| 4 | `create_numeric_buffer` | YES (`T`) | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn create_numeric_buffer` |
| 5 | `calculate_with_doubles`| YES (`T`) | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn calculate_with_doubles` |
| 6 | `doubleneg`             | YES (`T`) | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn doubleneg` |

**Symbol diff (C-defined minus Rust-defined): EMPTY.** No symbol needed a new
export wrapper and no C translation unit was skipped — `c_src` contains exactly
one source file (`src/lib.c`) and one header (`include/lib.h`), and every
non-`static` function in it is translated in `src/lib.rs`.

The Rust `.so` additionally exports Rust-mangled (`_ZN...`) internal symbols and
standard-library metadata symbols. Extra exports are harmless; the requirement is
that every C export exists in Rust, which holds.

This diff is asserted programmatically by `tests/phase_d_symbols.rs`
(`c_exports_are_subset_of_rust_exports`), which shells out to `nm -D` on both
libraries so the check cannot silently rot.

## Undefined (imported) symbols

The C `.so` imports only libc/libm: `memchr`, `pow`, `printf`, `puts` (plus the
usual weak `_ITM_*` / `__cxa_finalize` / `__gmon_start__`).

Note: GCC rewrites `printf("literal\n")` into `puts("literal")`, which is why
`puts` appears in the C import list but not in the C source. This is
byte-for-byte equivalent on stdout, so the Rust side keeping `printf` for those
calls produces identical output (verified by the stdout-differential tests in
`tests/phase_b_doubleneg.rs`).

The Rust `.so` imports the same libc/libm entry points it actually uses
(`memchr`, `pow`, `printf`, `trunc`, `memcpy`, `malloc`, ...) plus the Rust
standard library's runtime imports (`_Unwind_*`, `dl_iterate_phdr`,
`pthread_key_*`, `mmap64`, ...).

**0 missing / unresolvable non-libc symbols in the Rust `.so`.** Verified with:

```
ldd -r target/debug/libdoubleneg_lib.so   # no "undefined symbol" lines
```

asserted by `tests/phase_d_symbols.rs::rust_so_has_no_unresolved_symbols`.

## Deliberate non-goal: identical *internal* symbols

`double_to_int_trunc` in `src/lib.rs` is a private helper reproducing the x86-64
`cvttsd2si` semantics of C's `(int)double` cast. It is intentionally not
exported, because the C compiler emits that conversion inline (see the
`objdump -d` of `convert_double_to_int`: a single `cvttsd2si %xmm0,%eax`) rather
than as a callable symbol.
