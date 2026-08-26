# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   : `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`,
  no `CMAKE_BUILD_TYPE` → `-O0`, **assertions ENABLED** because `NDEBUG` is never
  defined)
* Rust: `target/debug/libconvert_pix_lib.so` (`crate-type = ["cdylib"]`)

## Build-time configuration surface

| source | configuration knobs | conclusion |
|--------|--------------------|------------|
| `Cargo.toml` | no `[features]` section at all (`cargo metadata` → `features == {}`) | exactly **one** feature combination exists: *no features* (`--no-default-features` == default) |
| `c_src/CMakeLists.txt` | no `option()`, no `target_compile_definitions`, single source file `src/lib.c` | one C configuration |
| `c_src/src/lib.c` / `include/lib.h` | `grep -n '#if\|#ifdef\|#ifndef\|#define\|NDEBUG'` → **no matches** | no preprocessor configuration at all |

Therefore Phase D's "repeat for every feature combination" collapses to the single
combination `--no-default-features` (verified identical to the default build).

## Symbol table

| # | symbol | C type/size | Rust type/size | present in Rust `.so` | notes |
|---|--------|-------------|----------------|-----------------------|-------|
| 1 | `cp_inflate`          | `T` 0x29b | `T` | yes | `#[no_mangle] pub unsafe extern "C" fn` |
| 2 | `convert_pix`         | `T` 0x1aa | `T` | yes | `#[no_mangle] pub unsafe extern "C" fn` |
| 3 | `cp_error_reason`     | `B` 8     | `B` 8     | yes | `static mut *const c_char` (`.bss`, initially NULL in both) |
| 4 | `cp_fixed_table`      | `D` 0x140 | `D` 0x140 | yes | `[u8; 288+32]` |
| 5 | `cp_permutation_order`| `D` 0x13  | `D` 0x13  | yes | `[u8; 19]` |
| 6 | `cp_len_extra_bits`   | `D` 0x1f  | `D` 0x1f  | yes | `[u8; 29+2]` |
| 7 | `cp_len_base`         | `D` 0x7c  | `D` 0x7c  | yes | `[u32; 29+2]` |
| 8 | `cp_dist_extra_bits`  | `D` 0x20  | `D` 0x20  | yes | `[u8; 30+2]` |
| 9 | `cp_dist_base`        | `D` 0x80  | `D` 0x80  | yes | `[u32; 30+2]` |

**Symbol diff (C-exported minus Rust-exported): EMPTY.**  All 9 defined dynamic
symbols of the C `.so` are exported by the Rust `.so` with the exact same name,
the exact same `nm` class (`T` for text, `D` for initialised data, `B` for
`.bss`) and byte-for-byte identical sizes for every data object.

Reproduce with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $2, $3}' | sort > /tmp/c.txt
nm -D --defined-only target/debug/libconvert_pix_lib.so | grep -v ' _ZN' \
  | awk '{print $2, $3}' | sort > /tmp/r.txt
diff /tmp/c.txt /tmp/r.txt   # -> no output
```

`tests/diff.rs` group `symbol_parity` performs the same comparison from inside
the test suite (it shells out to `nm -D` on both objects, ignoring Rust's private
`_ZN…` symbols) and group `cfg01_table_contents` additionally asserts that the
*contents* of all six exported tables — and the initial NULL of
`cp_error_reason` — are byte-identical when read through `dlsym` in a freshly
`dlopen`ed process.  Verified for the dev profile, `--no-default-features`, and
the release profile:

```
symbol_parity: 9 C symbols, 9 Rust symbols, diff empty = true
=== groups: 84  cases compared: 8015  identical aborts: 626  failures: 0 ===
ALL DIFFERENTIAL CHECKS PASSED
```

## Symbols intentionally NOT exported

These are `static` in `c_src/src/lib.c`, hence have internal linkage and appear in
neither `.so`'s dynamic symbol table.  They are translated in `src/lib.rs` as
private Rust `fn`s (so the code is complete), but must **not** be given
`#[no_mangle]`, otherwise the Rust `.so` would export symbols the C `.so` does
not:

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_paeth`,
`cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter`.

Verified absent from both objects:

```sh
for s in cp_build cp_decode cp_unfilter cp_paeth cp_stored cp_fixed cp_dynamic \
         cp_block cp_chunk cp_find cp_make32 cp_read_bits cp_peak_bits \
         cp_consume_bits cp_would_overflow cp_ptr cp_rev16 \
         cp_make_pixel cp_make_pixel_a; do
  nm -D --defined-only c_src/build/libtranslated_rust.so | grep -qw $s && echo "C exports $s"
  nm -D --defined-only target/debug/libconvert_pix_lib.so | grep -qw $s && echo "RUST exports $s"
done   # -> no output
```

No whole C module was skipped: `c_src/` contains exactly one translation unit
(`src/lib.c`, 493 lines) plus `include/lib.h`, and every function and every file
scope object in it has a counterpart in `src/lib.rs`:

| C (`c_src/src/lib.c`) | Rust (`src/lib.rs`) |
|---|---|
| `struct cp_pixel_t` (lib.h) | `pub struct cp_pixel_t` |
| `struct cp_image_t` | `pub struct cp_image_t` |
| `struct cp_state_t` | `struct cp_state_t` (`#[repr(C)]`, same 2464-byte layout / same field offsets `0x448` lit, `0x8c8` dst, `0x948` len, `0x994..0x99c` nlit/ndst/nlen) |
| `struct cp_raw_png_t` | `struct cp_raw_png_t` |
| `cp_make_pixel_a`, `cp_make_pixel` | `cp_make_pixel_a`, `cp_make_pixel` |
| `cp_would_overflow`, `cp_ptr`, `cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16` | same names |
| `cp_build`, `cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block` | same names (`cp_dynamic` additionally models the C stack frame — see CONFIGS.md) |
| `cp_inflate` | `cp_inflate` (`#[no_mangle] extern "C"`) |
| `cp_paeth`, `cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter` | same names (dead `static` code, kept for completeness) |
| `convert_pix` | `convert_pix` (`#[no_mangle] extern "C"`) |
| all 6 file-scope tables + `cp_error_reason` | `#[no_mangle] static mut` of the same type and size |
| `assert()` (glibc, `NDEBUG` undefined) | `cp_assert!` → `std::process::abort()` (same SIGABRT) |
| `memcpy`/`memset`/`memcmp`/`abs` | `ptr::copy_nonoverlapping` / `ptr::write_bytes` / `cp_memcmp` / `wrapping_abs` |
| `calloc`/`free` | `alloc_zeroed`/`dealloc` |

## Undefined (imported) symbols in the Rust `.so`

`nm -D -u target/debug/libconvert_pix_lib.so` lists only libc / libgcc-unwind /
libpthread entries (`memcpy`, `memset`, `memmove`, `malloc`, `calloc`, `free`,
`posix_memalign`, `abort`, `_Unwind_*`, `pthread_*`, `dl_iterate_phdr`, …) — i.e.
**0 missing non-libc symbols**.

## Notes on the two exported functions

`cp_inflate` and `convert_pix` are the only entry points.  All the arithmetic in
`src/lib.rs` uses `wrapping_*` operations, because C's signed overflow (e.g.
`in_bytes * 8`, `in_bytes - first_bytes` with `in_bytes == INT_MIN`) wraps in the
compiled C library while Rust's default `+`/`-`/`*` would *trap* in a debug
profile.  `abort_in_bytes_min_align{0..3}` and `abort_in_bytes_extremes` are the
regression tests for that: without the wrapping operations the Rust build died
with a panic message where the C build either aborted on
`assert(s->bits_left > 0)` or segfaulted.
