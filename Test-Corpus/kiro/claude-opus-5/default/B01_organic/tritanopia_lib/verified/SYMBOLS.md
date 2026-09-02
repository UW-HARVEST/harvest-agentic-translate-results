# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically, not from assumptions:

```
# C
cd c_src/build && nm -D --defined-only libharvest-work-0BhnKx.so
0000000000001670 T tritanopia

# Rust
cd translation && nm -D --defined-only target/release/libtritanopia_lib.so
0000000000011790 T tritanopia
```

## Symbol table

| # | symbol | C `.so` | Rust `.so` | C definition site | Rust definition site | status |
|---|--------|---------|------------|-------------------|----------------------|--------|
| 1 | `tritanopia` | `T` (global text) | `T` (global text) | `c_src/src/lib.c:55` | `translation/src/lib.rs` (`#[unsafe(no_mangle)] pub extern "C" fn tritanopia`) | MATCH |

**Symbol diff (C-exported minus Rust-exported): EMPTY.** No missing symbols, so
neither Phase A remedy (add a `#[no_mangle]` wrapper / translate an untranslated
module) is needed. No stubs, no `unimplemented!()`, no faked exports exist in the
Rust crate — verified by `grep -rn 'unimplemented\|todo!\|panic!' translation/src`.

## Non-exported C functions (`t`, file-local `static`) — completeness check

These are not part of the ABI, but each must still be *translated* (not skipped)
because `tritanopia` composes all of them. Presence verified in the Rust source:

| C static function | site | Rust counterpart | translated |
|-------------------|------|------------------|------------|
| `cbRemoveGammaRGB` | `lib.c:11` | `fn cbRemoveGammaRGB` | yes |
| `cbNorm`           | `lib.c:22` | `fn cbNorm`           | yes |
| `cbDenorm`         | `lib.c:28` | `fn cbDenorm` (+ `c_float_to_uchar`) | yes |
| `cbApplyGammaRGB`  | `lib.c:35` | `fn cbApplyGammaRGB`  | yes |
| `Tritanopia`       | `lib.c:48` | `fn Tritanopia`       | yes |

The C translation unit is a single file (`c_src/CMakeLists.txt` lists exactly
`src/lib.c`), so there is no untranslated module: the whole library is covered.

## Undefined (imported) symbols in the Rust `.so`

`nm -D -u target/release/libtritanopia_lib.so` lists only libc / libgcc-unwind
imports (`pow@GLIBC_2.29`, `memcpy`, `malloc`, `_Unwind_*`, …). `pow` is the same
libm entry point the C object imports (`nm -D -u` on the C `.so` also shows
`pow@GLIBC_2.29`), which is deliberate: the Rust code calls libm's `pow` via
`extern "C"` rather than `f64::powf`, so both builds resolve to the *same*
implementation and cannot disagree in the last bit.

**0 missing / 0 undefined non-libc symbols.**

## Types crossing the ABI

| C | Rust | notes |
|---|------|-------|
| `typedef struct cb_rgb_255 { unsigned char R, G, B; }` | `#[repr(C)] pub struct cb_rgb_255 { R, G, B: c_uchar }` | size 3, align 1; x86-64 SysV: one INTEGER eightbyte, passed/returned in `rdi`/`rax`. Confirmed against the C disassembly of `tritanopia`/`cbDenorm`. |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are therefore the complete set (both verified clean).
