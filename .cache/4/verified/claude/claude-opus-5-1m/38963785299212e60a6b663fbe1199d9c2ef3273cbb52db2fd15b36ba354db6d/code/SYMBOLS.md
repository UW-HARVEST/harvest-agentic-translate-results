# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How this was produced

```sh
# C shared library
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libtranslated_rust.so | sort

# Rust shared library (cdylib)
cargo build            # -> target/debug/libload_png_mem_lib.so
cargo build --release  # -> target/release/libload_png_mem_lib.so
nm -D --defined-only target/debug/libload_png_mem_lib.so | sort
```

The script `tests/symbols.rs::symbol_parity` performs this diff automatically
and fails if the C `.so` exports anything the Rust `.so` does not.

## Full public surface of the C `.so`

`c_src` contains exactly one translation unit (`src/lib.c`) and one public
header (`include/lib.h`). Nothing else exists to translate — there is no
skipped module. `nm -D --defined-only` reports 9 symbols:

| # | symbol | type | C declaration | present in Rust `.so` |
|---|--------|------|---------------|-----------------------|
| 1 | `load_png_mem`         | `T` (text) | `cp_image_t load_png_mem(const uint8_t *png_data, int png_length)` | yes |
| 2 | `cp_inflate`           | `T` (text) | `int cp_inflate(void *in, int in_bytes, void *out, int out_bytes)` | yes |
| 3 | `cp_error_reason`      | `B` (.bss) | `const char *cp_error_reason;`         | yes |
| 4 | `cp_fixed_table`       | `D` (.data) | `uint8_t cp_fixed_table[288 + 32]`    | yes |
| 5 | `cp_permutation_order` | `D` (.data) | `uint8_t cp_permutation_order[19]`    | yes |
| 6 | `cp_len_extra_bits`    | `D` (.data) | `uint8_t cp_len_extra_bits[29 + 2]`   | yes |
| 7 | `cp_len_base`          | `D` (.data) | `uint32_t cp_len_base[29 + 2]`        | yes |
| 8 | `cp_dist_extra_bits`   | `D` (.data) | `uint8_t cp_dist_extra_bits[30 + 2]`  | yes |
| 9 | `cp_dist_base`         | `D` (.data) | `uint32_t cp_dist_base[30 + 2]`       | yes |

Everything else in `lib.c` is `static` (`cp_make_pixel_a`, `cp_make_pixel`,
`cp_would_overflow`, `cp_ptr`, `cp_peak_bits`, `cp_consume_bits`,
`cp_read_bits`, `cp_rev16`, `cp_build`, `cp_stored`, `cp_fixed`, `cp_decode`,
`cp_dynamic`, `cp_block`, `cp_paeth`, `cp_make32`, `cp_chunk`, `cp_find`,
`cp_unfilter`, `cp_convert`, `cp_get_alpha_for_indexed_image`, `cp_depalette`,
`cp_get_chunk_byte_length`, `cp_out_size`) and therefore has no dynamic symbol.
No macro-generated symbols exist (`lib.c` defines no function-generating
macros).

## Result

```
$ nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort -u > c.syms
$ nm -D --defined-only target/release/libload_png_mem_lib.so | awk '{print $3}' | sort -u > r.syms
$ comm -23 c.syms r.syms          # exported by C, missing from Rust
$ echo $?
0                                  # empty -> 0 missing symbols
```

```
missing from Rust .so : 0   (9 of 9 present)
```

Verified by `./verify.sh` for the default (and only) feature combination, in
both the `dev` and the `release` cargo profile.

### Symbol *sizes* also match

A consumer can read a whole exported table, so `nm -S` sizes are compared too:

| symbol | C size | Rust size |
|--------|--------|-----------|
| `cp_dist_base`         | `0x80` | `0x80` |
| `cp_dist_extra_bits`   | `0x20` | `0x20` |
| `cp_error_reason`      | `0x08` | `0x08` |
| `cp_fixed_table`       | `0x140` | `0x140` |
| `cp_len_base`          | `0x7c` | `0x7c` |
| `cp_len_extra_bits`    | `0x1f` | `0x1f` |
| `cp_permutation_order` | `0x13` | `0x13` |

`cp_error_reason` is in `.bss` (`B`) in both; all six tables are in `.data`
(`D`) in both — i.e. they are *writable* in the Rust `.so` too, which
`ERRORS.md` rows 38 and 41 rely on.

## Undefined (imported) symbols

Both libraries import only libc / runtime symbols. The C `.so` additionally
imports `__assert_fail` because `c_src/CMakeLists.txt` sets **no**
`CMAKE_BUILD_TYPE` and never defines `NDEBUG`, i.e. **`assert()` is live in the
C shared library** and a failing assertion terminates the process with
`SIGABRT`. The Rust translation reproduces this: each of the 10 `assert()`s in
`lib.c` is mirrored by an explicit check that calls libc `abort()`
(`cp_assert_fail`), so the Rust `.so` imports `abort`.

Non-libc undefined symbols in the Rust `.so`: **0** (only glibc + the
`_Unwind_*` / `dl_iterate_phdr` personality symbols contributed by the Rust
standard library are present, all of which are resolved by `libgcc_s`/`libc`).

## Data-symbol content parity

Because all six tables are *writable*, non-`const` globals in C, a consumer can
read them through `dlsym`. `tests/symbols.rs::data_table_contents` reads all
6 tables (and `cp_error_reason`) from both `.so`s through `libloading` and
asserts byte-for-byte equality of the full arrays:

| symbol | length in bytes |
|--------|-----------------|
| `cp_fixed_table`       | 320 (`288 + 32` × `u8`) |
| `cp_permutation_order` | 19 (`19` × `u8`) |
| `cp_len_extra_bits`    | 31 (`29 + 2` × `u8`) |
| `cp_len_base`          | 124 (`29 + 2` × `u32`) |
| `cp_dist_extra_bits`   | 32 (`30 + 2` × `u8`) |
| `cp_dist_base`         | 128 (`30 + 2` × `u32`) |
| `cp_error_reason`      | 8 (one `const char *`, `NULL` before first call) |
