# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/lib<parent-dir-name>.so   (CMake derives the target name from
#    the directory ABOVE c_src, so the file name is environment specific;
#    the tests glob for `lib*.so` instead of hard-coding it.)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libfloat2half_lib.so
```

## C `.so` exported (defined) symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-Te7Ifm.so
00000000000010f9 T float2half
```

Total: **1** exported function symbol.

`m__base` and `m__shift` are `static` in `src/lib.c`, therefore they have
internal linkage and are deliberately NOT part of the dynamic symbol table.
They must NOT be exported by the Rust `.so` either (and are not — they are
private `static` items in `src/lib.rs`).

## Rust `.so` exported (defined) symbols

```
$ nm -D --defined-only translation/target/release/libfloat2half_lib.so
0000000000012220 T float2half
```

## Parity table

| # | C symbol | type | present in Rust `.so`? | Rust item | action taken |
|---|----------|------|------------------------|-----------|--------------|
| 1 | `float2half` | `T` (global text) | YES — exact name | `#[unsafe(no_mangle)] pub extern "C" fn float2half(f32) -> u16` | none needed |

## Symbol diff

```
$ diff <(nm -D --defined-only <c.so>  | awk '{print $3}' | sort) \
       <(nm -D --defined-only <rs.so> | awk '{print $3}' | sort)
(empty)
```

**Missing-from-Rust symbols: 0. Undefined non-libc symbols in Rust `.so`: 0.**

No C source file was left untranslated: `c_src` contains exactly one
translation unit (`src/lib.c`, 118 lines) and one public header
(`include/lib.h`, 3 lines), and both are fully represented in
`translation/src/lib.rs`. No stubs, no `unimplemented!()`.

## Non-exported internal state parity

Because the two lookup tables are the entire behaviour of the library, they
were also diffed mechanically (element-by-element, parsed out of both source
files) rather than only through the function's output:

| table | C declaration | Rust declaration | length | element-wise equal |
|-------|---------------|------------------|--------|--------------------|
| base  | `static uint16_t m__base[512]` | `static M_BASE: [u16; 512]` | 512 = 512 | YES |
| shift | `static uint8_t m__shift[512]`  | `static M_SHIFT: [u8; 512]` | 512 = 512 | YES |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** and `src/`
contains **no `cfg(feature = ...)`** attributes, so exactly one feature
combination exists (the empty/default one). `--no-default-features` and the
default build are therefore the same build; both are still exercised by
`run_all.sh` for completeness.

## Automated enforcement

Symbol parity is not only recorded here, it is asserted by
`symbol_parity_c_so_vs_rust_so` in `tests/phase_d_exhaustive.rs` (which shells
out to `nm -D --defined-only` on both `.so` files) and re-checked by
`run_all.sh` for both the debug and release profiles. `run_all.sh` additionally
verifies that the Rust `.so` has **0 undefined non-libc symbols** and that
`ldd -r` reports no unresolved symbols.
