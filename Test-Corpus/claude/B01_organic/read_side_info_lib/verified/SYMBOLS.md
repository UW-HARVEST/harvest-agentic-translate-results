# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`,
  no `CMAKE_BUILD_TYPE` ⇒ `-O0`, gcc 11.5.0)
* Rust `.so`: `target/debug/libread_side_info_lib.so`
  (`[lib] crate-type = ["cdylib"], name = "read_side_info_lib"`)

## Feature / configuration matrix

`Cargo.toml` has **no `[features]` section at all**, and `c_src/CMakeLists.txt`
defines no `option()`, no `target_compile_definitions`, and no `#ifdef`-driven
variants (`grep -c '#if' c_src/src/lib.c c_src/include/lib.h` ⇒ 0).

Therefore the **complete** set of valid build configurations is a single one:

| # | cargo invocation                                   | equivalent |
|---|----------------------------------------------------|------------|
| 1 | `cargo check` (default features = ∅)               | baseline   |
| 1 | `cargo check --no-default-features`                | identical to #1 (no default feature list exists) |
| 1 | `cargo check --all-features`                        | identical to #1 (no features exist) |

All three spellings are exercised by `scripts/check_all_features.sh`; they select
the same code because there is not a single `#[cfg(feature = ...)]` in `src/`
(`grep -c 'cfg(feature' src/lib.rs` ⇒ 0).

## Defined (exported) symbols

`nm -D --defined-only` on the C `.so`:

| # | symbol           | type | present in Rust `.so`? | notes |
|---|------------------|------|------------------------|-------|
| 1 | `read_side_info` | `T`  | **yes** (`T`)          | the only public symbol; declared in `c_src/include/lib.h` |

`nm -D --defined-only` on the Rust `.so`:

| # | symbol           | type | present in C `.so`? |
|---|------------------|------|---------------------|
| 1 | `read_side_info` | `T`  | yes                 |

**Symbol diff (C-defined − Rust-defined) = ∅.** Verified automatically by
`tests/phase_d_symbols.rs::c_defined_symbols_all_exported_by_rust`.

`get_bits` is `static` in the C translation unit, so it is deliberately *not*
exported by either object (`nm -D | grep get_bits` ⇒ empty for both). The Rust
keeps it as a private `unsafe fn`. Likewise the three scalefactor-band tables
are function-local `static const` in the C (`nm` shows them as local `r`
symbols `g_scf_long.2`, `g_scf_short.1`, `g_scf_mixed.0`) and are private
statics in Rust — neither object exports them, so there is nothing to add.

No C source file in `c_src/` is left untranslated: `c_src/src/` contains exactly
one file (`lib.c`, 163 lines) and every one of its two functions
(`get_bits`, `read_side_info`) plus all three data tables exist in `src/lib.rs`.

## Undefined symbols in the Rust `.so`

`nm -D -u` on the Rust object lists only libc / libgcc-unwind imports
(`malloc`, `memcpy`, `_Unwind_*`, `dl_iterate_phdr`, …) that come from the Rust
standard library. **0 non-libc undefined symbols**, i.e. nothing that a C
consumer would have to provide. Checked by
`tests/phase_d_symbols.rs::rust_has_no_unexpected_undefined_symbols`.

## Result

```
$ nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort
read_side_info
$ nm -D --defined-only target/debug/libread_side_info_lib.so | awk '{print $NF}' | sort
read_side_info
$ nm -D --defined-only target/release/libread_side_info_lib.so | awk '{print $NF}' | sort
read_side_info
```

Symbol diff (C − Rust) is **empty** for both the dev-profile and the
release-profile Rust object. Nothing was stubbed: `read_side_info` is a real
translation of the C body, and no C source file was left untranslated
(`c_src/src/` = `lib.c` only).
