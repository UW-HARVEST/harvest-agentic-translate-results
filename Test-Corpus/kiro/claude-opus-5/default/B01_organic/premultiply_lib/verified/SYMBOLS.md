# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-ZXCGDf.so   (name derives from parent dir name)

cd translation && cargo build && cargo build --release
# -> translation/target/{debug,release}/libpremultiply_lib.so
```

## C source surface (ground truth)

`c_src/` contains exactly two source files, both fully accounted for:

| file | lines | public declarations |
|------|-------|---------------------|
| `c_src/include/lib.h` | 16 | `cp_pixel_t`, `cp_image_t`, `void premultiply(cp_image_t *img)` |
| `c_src/src/lib.c` | 20 | definition of `premultiply` |

There are no other translation units, no macro-generated symbol names, no
`#ifdef` feature gates, and no namespace-renaming macros in the header, so each
linker symbol equals its source-level name.

## Exported (defined, dynamic) symbols

`nm -D --defined-only` on each `.so`:

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `premultiply` | `T` | `T` | PRESENT in both — exact name match |

Symbol diff (C exported ⟶ missing from Rust): **EMPTY**.

```
$ comm -23 <(nm -D --defined-only c_src/build/libharvest-work-ZXCGDf.so \
             | awk '{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/libpremultiply_lib.so \
             | awk '{print $3}' | sort)
(no output)
```

No symbol required a new `#[no_mangle]` wrapper and no C module was left
untranslated: the single C translation unit (`src/lib.c`) is fully represented
by `translation/src/lib.rs`.

## Undefined (imported) symbols

| `.so` | undefined symbols |
|-------|-------------------|
| C | 4, all weak toolchain/libc hooks: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC`, `__gmon_start__` |
| Rust | 48, all libc (`malloc`, `memcpy`, `mmap64`, `write`, …), libgcc unwinder (`_Unwind_*@GCC_*`), and weak toolchain hooks. These come from the Rust standard library's panic/backtrace machinery. |

**0 missing/undefined non-libc symbols in the Rust `.so`.** Every `U`/`w` entry
in the Rust object resolves against `libc`/`libgcc_s`, which the loader supplies;
`libloading` opens the object successfully, which is the practical confirmation
(all differential tests dlopen it).

## Types crossing the ABI

Verified layout-compatible; both are `#[repr(C)]` in Rust.

| C | size | align | Rust |
|---|------|-------|------|
| `cp_pixel_t { uint8_t r,g,b,a; }` | 4 | 1 | `cp_pixel_t { r: u8, g: u8, b: u8, a: u8 }` |
| `cp_image_t { int w; int h; cp_pixel_t *pix; }` | 16 (4+4+pad0+8) | 8 | `cp_image_t { w: c_int, h: c_int, pix: *mut cp_pixel_t }` |

`sizeof(cp_pixel_t) == 4` is the constant the C loop uses as its stride
multiplier and increment; the Rust translation hardcodes `PIXEL_SIZE = 4`, which
matches.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
buildable configuration is the default one (`--no-default-features` and the
default build are the same compilation). Phase D's "every feature combination"
requirement collapses to a single combination, which is verified explicitly by
`tests/feature_matrix.rs` (it enumerates features from `Cargo.toml` and asserts
the set is empty, so no combination is silently skipped).

There *is* a second real configuration axis, though: the crate sets
`panic = "abort"` under `[profile.release]`, so the debug and release cdylibs are
genuinely different artifacts, and debug additionally enables rustc's `ub_checks`.
`verify.sh` therefore runs the entire suite twice, once against
`target/debug/libpremultiply_lib.so` and once against
`target/release/libpremultiply_lib.so` (selected with the `DIFF_RUST_SO`
environment variable). This mattered: **both divergences recorded in `ERRORS.md`
reproduced only under the debug cdylib** — the release build happened to match,
so a release-only run would have declared the translation correct.

## Reproducing

```
cd translation && ./verify.sh          # full gate (adds ~13 min of large-span tests)
cd translation && ./verify.sh --fast   # everything except the multi-GiB spans
```
