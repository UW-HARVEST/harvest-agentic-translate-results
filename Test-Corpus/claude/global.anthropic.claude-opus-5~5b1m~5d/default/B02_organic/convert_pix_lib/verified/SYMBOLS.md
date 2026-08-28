# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  : `c_src/build/libharvest-work-VVQnEx.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust: `translation/target/release/libconvert_pix_lib.so`
  (built with `cargo build --release`)

## C source inventory

`c_src` contains exactly two source files, both translated:

| C file | contents | translated in |
|--------|----------|---------------|
| `c_src/include/lib.h` | `cp_pixel_t`, `convert_pix` prototype | `src/lib.rs` |
| `c_src/src/lib.c`     | all 9 exported symbols + 17 `static` helpers | `src/lib.rs` |

No C module was skipped, so no symbol needed a new translation; every symbol
below has a real implementation (no stubs, no `unimplemented!()`).

## Symbol table

| # | symbol | nm type (C) | C decl | in Rust `.so`? | Rust definition |
|---|--------|-------------|--------|----------------|-----------------|
| 1 | `convert_pix`          | `T` (text)   | `void convert_pix(int bpp, int w, int h, uint8_t *src, cp_pixel_t *dst)` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn convert_pix` |
| 2 | `cp_inflate`           | `T` (text)   | `int cp_inflate(void *in, int in_bytes, void *out, int out_bytes)`       | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn cp_inflate` |
| 3 | `cp_error_reason`      | `B` (.bss)   | `const char *cp_error_reason;`        | yes | `#[unsafe(no_mangle)] pub static mut cp_error_reason: *const c_char` |
| 4 | `cp_fixed_table`       | `D` (.data)  | `uint8_t cp_fixed_table[288 + 32]`   | yes | `#[unsafe(no_mangle)] pub static mut cp_fixed_table: [u8; 320]` |
| 5 | `cp_permutation_order` | `D` (.data)  | `uint8_t cp_permutation_order[19]`   | yes | `#[unsafe(no_mangle)] pub static mut cp_permutation_order: [u8; 19]` |
| 6 | `cp_len_extra_bits`    | `D` (.data)  | `uint8_t cp_len_extra_bits[29 + 2]`  | yes | `#[unsafe(no_mangle)] pub static mut cp_len_extra_bits: [u8; 31]` |
| 7 | `cp_len_base`          | `D` (.data)  | `uint32_t cp_len_base[29 + 2]`       | yes | `#[unsafe(no_mangle)] pub static mut cp_len_base: [u32; 31]` |
| 8 | `cp_dist_extra_bits`   | `D` (.data)  | `uint8_t cp_dist_extra_bits[30 + 2]` | yes | `#[unsafe(no_mangle)] pub static mut cp_dist_extra_bits: [u8; 32]` |
| 9 | `cp_dist_base`         | `D` (.data)  | `uint32_t cp_dist_base[30 + 2]`      | yes | `#[unsafe(no_mangle)] pub static mut cp_dist_base: [u32; 32]` |

Result: **0 missing symbols.** Verified automatically by
`tests/symbols.rs::symbol_parity_c_so_vs_rust_so`, which shells out to `nm -D`
on both objects and asserts the C set minus the Rust set is empty.

## `static` (non-exported) C helpers — translated but intentionally not exported

These have internal linkage in C, so they must NOT appear in `nm -D`.  They are
translated as private Rust `fn`s and are exercised indirectly through
`cp_inflate` (or, for the PNG helpers, are dead code in both objects because
the C file that used them was reduced to `convert_pix`).

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_paeth`,
`cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter`.

## Undefined (imported) symbols

The C object imports only libc: `__assert_fail`, `calloc`, `free`, `memcmp`,
`memcpy`, `memset` (plus the usual weak `_ITM_*` / `__gmon_start__` /
`__cxa_finalize`).  The Rust object imports libc equivalents through the Rust
standard library; there are no undefined non-libc symbols in either object.

## Note: the C's `.data` image is part of the ABI, not just the symbol names

`cp_block` indexes `cp_len_extra_bits` / `cp_len_base` with `symbol - 257` and
`cp_dist_extra_bits` / `cp_dist_base` with `distance_symbol`, and range-checks
neither.  The out-of-bounds case is reachable (see `ERRORS.md` row 31), so the C
reads whatever the linker put next in `.data`:

```
readelf -SW  c_src/build/lib*.so   # .data @ 0x5040 size 0x2a0, .bss @ 0x52e0 size 0x10
readelf -sW  c_src/build/lib*.so
  0x5040 cp_fixed_table        320
  0x5180 cp_permutation_order   19   (+13 padding)
  0x51a0 cp_len_extra_bits      31   (+1  padding)
  0x51c0 cp_len_base           124   (+4  padding)
  0x5240 cp_dist_extra_bits     32
  0x5260 cp_dist_base          128   -> .data ends at 0x52e0
  0x52e0 (.bss) libc `completed.0`
  0x52e8 (.bss) cp_error_reason
```

Rust orders and pads its statics differently:

```
nm -D target/release/libconvert_pix_lib.so
  cp_dist_extra_bits, cp_fixed_table, cp_len_extra_bits, cp_permutation_order,
  cp_dist_base, cp_len_base          # different order, no inter-object padding
```

so `cp_len_extra_bits[31]` would read `cp_permutation_order[0] == 16` in Rust but
a padding `0` in C.  `src/lib.rs` therefore routes those four reads through
`cp_data_byte()` / `cp_data_u32()`, which reconstruct the C's `.data`/`.bss`
image at the documented offsets.  This is not visible in `nm -D`, but it is part
of what an external caller can observe, so it is verified by
`tests/oob_tables.rs`.
