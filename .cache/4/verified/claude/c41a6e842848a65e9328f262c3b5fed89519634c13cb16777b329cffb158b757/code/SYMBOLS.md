# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build c_src/build
cargo build --offline
nm -D -S --defined-only c_src/build/libtranslated_rust.so | sort -k4
nm -D -S --defined-only target/debug/libunfilter_lib.so     | sort -k4
```

The C translation unit is a single file (`c_src/src/lib.c`, 478 lines) built into
`libtranslated_rust.so`. Everything declared `static` there has internal linkage
and therefore does **not** appear in `nm -D`; only the 2 functions and 7 data
objects below are part of the ABI.

## Dynamic symbols exported by the C `.so` (all of them)

| # | symbol | type | size (C) | size (Rust) | present in Rust `.so` | Rust definition |
|---|--------|------|----------|-------------|-----------------------|-----------------|
| 1 | `cp_inflate`           | `T` FUNC   | 0x29b | 0xdff | yes | `src/inflate.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn cp_inflate` |
| 2 | `unfilter`             | `T` FUNC   | 0x470 | 0x1aed | yes | `src/unfilter.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn unfilter` |
| 3 | `cp_error_reason`      | `B` OBJECT | 0x08  | 0x08  | yes | `src/tables.rs` `static mut cp_error_reason` |
| 4 | `cp_fixed_table`       | `D` OBJECT | 0x140 | 0x140 | yes | `src/tables.rs` `static mut cp_fixed_table` |
| 5 | `cp_permutation_order` | `D` OBJECT | 0x13  | 0x13  | yes | `src/tables.rs` `static mut cp_permutation_order` |
| 6 | `cp_len_extra_bits`    | `D` OBJECT | 0x1f  | 0x1f  | yes | `src/tables.rs` `static mut cp_len_extra_bits` |
| 7 | `cp_len_base`          | `D` OBJECT | 0x7c  | 0x7c  | yes | `src/tables.rs` `static mut cp_len_base` |
| 8 | `cp_dist_extra_bits`   | `D` OBJECT | 0x20  | 0x20  | yes | `src/tables.rs` `static mut cp_dist_extra_bits` |
| 9 | `cp_dist_base`         | `D` OBJECT | 0x80  | 0x80  | yes | `src/tables.rs` `static mut cp_dist_base` |

Function *code* sizes legitimately differ (different compilers); every **data
object size matches byte-for-byte**, and `tests/symbols.rs` additionally
verifies that the initial *contents* of all 7 objects are byte-identical
through `dlsym`.

## Missing symbols

**None.** Diff of the two `nm -D --defined-only` name lists is empty:

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/debug/libunfilter_lib.so   | awk '{print $NF}' | sort)
(no output)
```

Verified automatically by `tests/symbols.rs::c_and_rust_export_the_same_symbols`.

## `static` (internal-linkage) C entities — not part of the ABI, but translated

These have no `nm -D` entry in either library. They are listed for completeness
of the *translation* (Phase A rule: nothing may be absent because a whole piece
of C source was skipped):

| C entity (all `static` / file-local) | Rust counterpart |
|--------------------------------------|------------------|
| `struct cp_pixel_t`                          | `src/misc.rs` `CpPixel` |
| `struct cp_image_t`                          | `src/misc.rs` `CpImage` |
| `cp_make_pixel_a()`                          | `src/misc.rs` `cp_make_pixel_a` |
| `cp_make_pixel()`                            | `src/misc.rs` `cp_make_pixel` |
| `struct cp_raw_png_t`                        | `src/misc.rs` `CpRawPng` |
| `cp_make32()`                                | `src/misc.rs` `cp_make32` |
| `cp_chunk()`                                 | `src/misc.rs` `cp_chunk` |
| `cp_find()`                                  | `src/misc.rs` `cp_find` |
| `struct cp_state_t`                          | `src/inflate.rs` `CpState` |
| `cp_would_overflow()`                        | `src/inflate.rs` `cp_would_overflow` |
| `cp_ptr()`                                   | `src/inflate.rs` `cp_ptr` |
| `cp_peak_bits()`                             | `src/inflate.rs` `cp_peak_bits` |
| `cp_consume_bits()`                          | `src/inflate.rs` `cp_consume_bits` |
| `cp_read_bits()`                             | `src/inflate.rs` `cp_read_bits` |
| `cp_rev16()`                                 | `src/inflate.rs` `cp_rev16` |
| `cp_build()`                                 | `src/inflate.rs` `cp_build` |
| `cp_stored()`                                | `src/inflate.rs` `cp_stored` |
| `cp_fixed()`                                 | `src/inflate.rs` `cp_fixed` |
| `cp_decode()`                                | `src/inflate.rs` `cp_decode` |
| `cp_dynamic()`                               | `src/inflate.rs` `cp_dynamic` |
| `cp_block()`                                 | `src/inflate.rs` `cp_block` |
| `cp_paeth()`                                 | `src/unfilter.rs` `cp_paeth` |

## Undefined (imported) symbols

C imports only libc: `__assert_fail`, `calloc`, `free`, `memcmp`, `memcpy`,
`memset` (+ the usual weak `__cxa_finalize`, `__gmon_start__`,
`_ITM_*Table`). The Rust `.so` imports libc/`std` equivalents. No non-libc
undefined symbols in either library.

## Verification status

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/debug/libunfilter_lib.so    | awk '{print $NF}' | sort)
(empty)
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/release/libunfilter_lib.so  | awk '{print $NF}' | sort)
(empty)
```

* [x] 0 symbols missing from the Rust `.so` (debug **and** release profile).
* [x] All 7 data objects have identical `nm -S` sizes *and* identical contents.
* [x] 0 undefined non-libc symbols in either library
      (`nm -D -u` on the Rust `.so` lists only glibc/libgcc imports).
* [x] Nothing had to be stubbed: the whole translation unit was already
      translated, including the `static` helpers that are dead code in both
      libraries.

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** ⇒ the only valid feature
  combination is the empty one. `cargo check/test --no-default-features` and
  plain `cargo check/test` are therefore the same configuration, and both are
  exercised by `scripts/check_all_features.sh`.
* `c_src/CMakeLists.txt` has no `option()`/`add_definitions()`/`#ifdef`
  configuration either: one source file, `SHARED`, links `m`. In particular
  **`NDEBUG` is never defined**, so every `assert()` in `lib.c` is live — this is
  the reason `ERRORS.md` contains abort rows.
