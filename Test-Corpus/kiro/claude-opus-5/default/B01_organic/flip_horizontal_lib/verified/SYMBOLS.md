# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:

* C   : `c_src/build/libharvest-work-e9Wc2B.so`
* Rust: `translation/target/release/libflip_horizontal_lib.so`

## C exported (defined) symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-e9Wc2B.so
00000000000010f9 T flip_horizontal
```

## Rust exported (defined) symbols

```
$ nm -D --defined-only translation/target/release/libflip_horizontal_lib.so
00000000000116e0 T flip_horizontal
```

## Parity table

| # | C symbol          | type | present in Rust `.so` | notes |
|---|-------------------|------|-----------------------|-------|
| 1 | `flip_horizontal` | `T`  | YES (`T`)             | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |

## Symbol diff

```
$ diff <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only Rust.so | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust: 0. Extra in Rust: 0. Undefined non-libc in Rust: 0.**

The C library is a single translation unit (`src/lib.c`) with a single public
header (`include/lib.h`). The header declares no macros, no renaming/aliasing,
and no additional entry points, so no macro-generated symbols exist and no C
module was left untranslated. Nothing to add or translate.

## Non-symbol ABI surface (types crossing the boundary)

| C type        | definition                                    | Rust mirror        | layout check |
|---------------|-----------------------------------------------|--------------------|--------------|
| `cp_pixel_t`  | `uint8_t r,g,b,a`                             | `#[repr(C)] cp_pixel_t` | C: size 4, align 1 (verified with `sizeof`/`_Alignof`) |
| `cp_image_t`  | `int w; int h; cp_pixel_t *pix;`              | `#[repr(C)] cp_image_t` | C: size 16, align 8, offsets w=0 h=4 pix=8 |

These are verified at runtime by the differential tests, which declare their own
independent `#[repr(C)]` caller-side struct (in `tests/common/mod.rs`) so a
layout mistake inside the crate cannot hide itself, and which compare the raw
struct bytes after each call.

## Undefined-symbol check

`nm -D --undefined-only` on the Rust `.so` resolves entirely to glibc
(`@GLIBC_*`), the libgcc unwinder (`_Unwind_*@GCC_*`), and the standard weak
toolchain hooks (`_ITM_*TMCloneTable`, `__gmon_start__`, `__tls_get_addr`).
**0 undefined non-toolchain symbols.** Checked automatically by
`scripts/verify_all.sh` for every feature combination.

## Automation

`scripts/verify_all.sh` rebuilds the C library, enumerates every feature
combination from `Cargo.toml`, and for each one rebuilds the release cdylib,
requires an EMPTY `nm -D` diff against the C `.so`, and runs the whole
differential suite. Current result: **ALL COMBINATIONS PASSED** (2 combinations
— `default` and `--no-default-features`; `Cargo.toml` declares no `[features]`
table, so those are the only two that exist).
