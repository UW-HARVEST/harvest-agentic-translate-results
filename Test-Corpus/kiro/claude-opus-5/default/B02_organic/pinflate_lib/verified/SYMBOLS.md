# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D c_src/build/libharvest-work-wGYayD.so | grep -v ' U \| w ' | awk '{print $3}' | sort > /tmp/c.txt

cd translation && cargo build --release
nm -D translation/target/release/libpinflate_lib.so | grep -v ' U \| w ' | awk '{print $3}' | sort > /tmp/r.txt

diff /tmp/c.txt /tmp/r.txt      # MUST be empty
```

`tests/symbol_parity.rs` performs exactly this diff as a test.

## C source → exported symbols

`c_src/src/lib.c` is a single translation unit. Everything declared `static`
is internal and contributes no dynamic symbol. The non-`static` file-scope
objects and the one non-`static` function are the entire exported surface.

| # | symbol | kind (`nm`) | C declaration | in Rust `.so`? | Rust item |
|---|--------|-------------|---------------|----------------|-----------|
| 1 | `pinflate`            | `T` text   | `int pinflate(void *in, int in_bytes, void *out, int out_bytes)` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn pinflate` |
| 2 | `cp_error_reason`     | `B` bss    | `const char *cp_error_reason;`        | yes | `pub static mut cp_error_reason: *const c_char` |
| 3 | `cp_fixed_table`      | `D` data   | `uint8_t cp_fixed_table[288 + 32]`    | yes | `pub static mut cp_fixed_table: [u8; 320]` |
| 4 | `cp_permutation_order`| `D` data   | `uint8_t cp_permutation_order[19]`    | yes | `pub static mut cp_permutation_order: [u8; 19]` |
| 5 | `cp_len_extra_bits`   | `D` data   | `uint8_t cp_len_extra_bits[29 + 2]`   | yes | `pub static mut cp_len_extra_bits: [u8; 31]` |
| 6 | `cp_len_base`         | `D` data   | `uint32_t cp_len_base[29 + 2]`        | yes | `pub static mut cp_len_base: [u32; 31]` |
| 7 | `cp_dist_extra_bits`  | `D` data   | `uint8_t cp_dist_extra_bits[30 + 2]`  | yes | `pub static mut cp_dist_extra_bits: [u8; 32]` |
| 8 | `cp_dist_base`        | `D` data   | `uint32_t cp_dist_base[30 + 2]`       | yes | `pub static mut cp_dist_base: [u32; 32]` |

**Missing from Rust `.so`: none.** No C module/file was skipped —
`c_src/CMakeLists.txt` compiles exactly one source file (`src/lib.c`) and every
non-`static` object in it is exported by the Rust `cdylib` under the identical
name. No stubs / `unimplemented!()` were introduced.

## Internal (`static`) C functions — translated, intentionally not exported

These have no dynamic symbol in the C `.so` either, so exporting them would be
a *parity violation*. All are present as private Rust `fn`s and are exercised
transitively through `pinflate`.

| C `static` function | Rust counterpart |
|---------------------|------------------|
| `cp_make_pixel_a`   | `fn cp_make_pixel_a` (dead in C too) |
| `cp_make_pixel`     | `fn cp_make_pixel` (dead in C too) |
| `cp_would_overflow` | `fn cp_would_overflow` |
| `cp_ptr`            | `unsafe fn cp_ptr` |
| `cp_peak_bits`      | `unsafe fn cp_peak_bits` |
| `cp_consume_bits`   | `fn cp_consume_bits` |
| `cp_read_bits`      | `unsafe fn cp_read_bits` |
| `cp_rev16`          | `fn cp_rev16` |
| `cp_build`          | `unsafe fn cp_build` |
| `cp_stored`         | `unsafe fn cp_stored` |
| `cp_fixed`          | `unsafe fn cp_fixed` |
| `cp_decode`         | `unsafe fn cp_decode` |
| `cp_dynamic`        | `unsafe fn cp_dynamic` |
| `cp_block`          | `unsafe fn cp_block` |

Types `struct cp_pixel_t`, `struct cp_image_t`, `struct cp_state_t` are
translated as `#[repr(C)]` structs with the original field order, because
`cp_decode` performs an out-of-bounds `tree[lo - 1]` read that lands on the
*neighbouring field* of `cp_state_t` (e.g. `lookup[510..512]` when
`tree == s->lit`); identical layout is required to reproduce it.

## Data-layout parity (not just symbol-name parity)

Exporting the right *names* is not sufficient — the reference `.so` is also
indexed **out of bounds** through them, so their relative addresses are part of
the observable behaviour. Measured with `dlsym` on both libraries:

| symbol | offset in C `.so` | offset in Rust `.so` (as first written) |
|--------|------------------:|----------------------------------------:|
| `cp_fixed_table`       |   0 |    0 |
| `cp_permutation_order` | 320 |  351 |
| `cp_len_extra_bits`    | 352 |  320 |
| `cp_len_base`          | 384 |  500 |
| `cp_dist_extra_bits`   | 512 |  -32 |
| `cp_dist_base`         | 544 |  372 |

Because `cp_block` evaluates `cp_len_extra_bits[symbol]`, `cp_len_base[symbol]`,
`cp_dist_extra_bits[sym]` and `cp_dist_base[sym]` with values that come out of
`cp_decode` (and a corrupted Huffman tree can make those reach 4095), the C
reads its `.data` *neighbours*, and the scrambled Rust order produced different
bytes. This was a real divergence found by `tests/fuzz.rs`. It is fixed by
routing every table read through `CP_SHADOW`, an internal `#[repr(C)]` struct
whose field offsets reproduce the C's layout exactly (including the 13/1/4/8
byte alignment gaps and the `cp_error_reason` slot at +680), refreshed from the
writable exports on each entry to `pinflate`. The offsets are asserted at
compile time in `src/lib.rs`.

## Undefined (imported) symbols

The C `.so` imports `calloc`, `free`, `memcpy`, `memset`, `__assert_fail` from
libc plus the usual weak ELF/glibc hooks. The Rust `.so` imports only libc /
`libgcc_s` runtime symbols. `nm -D --undefined-only` on the Rust `.so` shows **0
non-libc undefined symbols** — verified by `tests/symbol_parity.rs`.

## Build-configuration note (drives Phase C)

`c_src/CMakeLists.txt` sets **no** `CMAKE_BUILD_TYPE` and no `-DNDEBUG`, so the
16 `assert()`s in `lib.c` are **live** in the reference `.so`
(`nm -D` shows `U __assert_fail@GLIBC_2.2.5`). Malformed input therefore makes
the C library `abort()` (`SIGABRT`) rather than return. The Rust translation
reproduces every one of those assertions via `cp_assert_fail() -> !` which calls
`std::process::abort()`, so both libraries die with the same signal on the same
inputs. See `ERRORS.md` rows A1–A9.
