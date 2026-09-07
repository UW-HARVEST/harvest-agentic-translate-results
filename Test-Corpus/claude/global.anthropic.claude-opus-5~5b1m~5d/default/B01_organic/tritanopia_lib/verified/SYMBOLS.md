# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
c_src:      cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
            -> c_src/build/libharvest-work-bRg58z.so
translation: cargo build --release
            -> translation/target/release/libtritanopia_lib.so
```

## C `.so` exported (defined) symbols

`nm -D --defined-only c_src/build/libharvest-work-bRg58z.so`

```
0000000000001670 T tritanopia
```

That is the complete list. Every other function in `c_src/src/lib.c`
(`cbRemoveGammaRGB`, `cbNorm`, `cbDenorm`, `cbApplyGammaRGB`, `Tritanopia`) is
declared `static`, i.e. internal linkage, and is therefore deliberately absent
from the dynamic symbol table. `c_src/include/lib.h` declares exactly one
function and contains no namespacing/renaming macros, so no macro-generated
symbol names exist.

## Rust `.so` exported (defined) symbols

`nm -D --defined-only translation/target/release/libtritanopia_lib.so`

```
0000000000011800 T tritanopia
```

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | type | status |
|---|--------|-----------|---------------|------|--------|
| 1 | `tritanopia` | yes (`T`) | yes (`T`) | `cb_rgb_255 (cb_rgb_255)` | MATCH |

**Missing from Rust: none.** **Extra in Rust: none.**

No module of the C was skipped: `c_src/src/lib.c` is the only translation unit
(see `add_library(... src/lib.c)` in `c_src/CMakeLists.txt`) and all five of its
`static` helpers have corresponding private Rust functions
(`cbRemoveGammaRGB`, `cbNorm`, `cbDenorm`, `cbApplyGammaRGB`, `Tritanopia`) in
`translation/src/lib.rs`. Nothing is stubbed or `unimplemented!()`.

## Undefined (imported) symbols

Only libc/libm imports. The C imports `pow` from `libm`; the Rust binds the same
platform `pow` via `#[link(name = "m")] extern "C" { fn pow(f64,f64)->f64; }`
rather than `f64::powf`, so both observe bit-identical transcendental results.
There are no non-libc undefined symbols in the Rust `.so`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. `cargo test --no-default-features`
is equivalent to `cargo test`. Both are exercised anyway (see `run_all.sh`).

## ABI note

`cb_rgb_255` is a 3-byte aggregate. Under the x86-64 SysV ABI it is classified
INTEGER and passed/returned in a single general-purpose register (`%rdi` in,
`%rax` out — confirmed in the C disassembly, which does
`movzbl -0x38(%rbp),%edx` etc. on the spilled register). The Rust
`#[repr(C)] struct cb_rgb_255 { u8, u8, u8 }` behind `extern "C"` follows the
same platform rule. Only the low 3 bytes of the register are architecturally
meaningful; the upper 5 bytes are unspecified padding on both sides, so the
differential tests compare the 3 defined bytes.
