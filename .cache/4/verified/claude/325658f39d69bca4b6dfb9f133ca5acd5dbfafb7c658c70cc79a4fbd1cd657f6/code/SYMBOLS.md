# SYMBOLS.md — symbol parity (Phase A / Phase D)

## Build configurations

`Cargo.toml` has **no `[features]` table**, and the library C source
(`c_src/src/lib.c`, `c_src/include/lib.h`) contains **no `#ifdef` / `#if` /
`-D` conditional compilation** (`c_src/CMakeLists.txt` passes no `target_compile_definitions`).

Therefore there is exactly **one** valid build configuration:

| # | combo | command |
|---|-------|---------|
| 1 | *(default = empty feature set)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

`cargo check --no-default-features` → clean (no errors, no warnings).

## C `.so` exported (defined, dynamic) symbols

`nm -D --defined-only c_src/build/libtranslated_rust.so`

| symbol | type | present in Rust `.so`? |
|--------|------|------------------------|
| `dequantize_granule` | `T` (global text) | **YES** (`#[unsafe(no_mangle)] pub unsafe extern "C" fn dequantize_granule`) |

Total C exported symbols: **1**. Total missing from Rust: **0**.

`get_bits` is `static` in `lib.c`, so it is deliberately *not* exported by
either object (it is a private `unsafe fn` in Rust). Verified absent from both
`nm -D` listings — exporting it would be a parity *violation*.

## Rust `.so` exported (defined, dynamic) symbols

`nm -D --defined-only target/release/libdequantize_granule_lib.so`

| symbol | type |
|--------|------|
| `dequantize_granule` | `T` |

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so    | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libdequantize_granule_lib.so | awk '{print $3}' | sort)
(empty)
```

**Diff is EMPTY — full parity in both directions.**

## Undefined symbols

Both objects import only weak/`libc`/toolchain symbols. C imports
`_ITM_*`, `__cxa_finalize`, `__gmon_start__`. Rust additionally imports
`_Unwind_*` (panic machinery) and libc (`malloc`, `memcpy`, `read`, ...).

**0 missing/undefined non-libc symbols in the Rust `.so`.**

## Translation completeness

`c_src` is a single 43-line translation unit. Every C construct in it has a
Rust counterpart — nothing was skipped, nothing is stubbed and there is no
`unimplemented!()`/`todo!()` anywhere in `src/`:

| C entity (`c_src/src/lib.c`) | Rust counterpart (`src/lib.rs`) | linkage |
|---|---|---|
| `typedef struct {...} bs_t` (`lib.h:3`) | `#[repr(C)] pub struct bs_t` | type only |
| `typedef struct {...} L12_scale_info` (`lib.h:8`) | `#[repr(C)] pub struct L12_scale_info` | type only |
| `static uint32_t get_bits(bs_t*, int)` (`lib.c:3`) | `unsafe fn get_bits` | private (matches `static`) |
| `int dequantize_granule(float*, bs_t*, L12_scale_info*, int)` (`lib.c:17`) | `pub unsafe extern "C" fn dequantize_granule` | exported |
