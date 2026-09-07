# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-6DBXiQ.so
nm -D --defined-only translation/target/release/libconvert_pix_lib.so
```

## C `.so` exported (defined) symbols → Rust `.so`

| # | symbol | C type | Rust `.so` | notes |
|---|--------|--------|-----------|-------|
| 1 | `cp_inflate` | `T` (text/func) | present `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn cp_inflate` |
| 2 | `convert_pix` | `T` (text/func) | present `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn convert_pix` |
| 3 | `cp_fixed_table` | `D` (data, 320 B) | present `D` | `pub static mut cp_fixed_table: [u8; 320]` |
| 4 | `cp_permutation_order` | `D` (data, 19 B) | present `D` | `pub static mut cp_permutation_order: [u8; 19]` |
| 5 | `cp_len_extra_bits` | `D` (data, 31 B) | present `D` | `pub static mut cp_len_extra_bits: [u8; 31]` |
| 6 | `cp_len_base` | `D` (data, 124 B) | present `D` | `pub static mut cp_len_base: [u32; 31]` |
| 7 | `cp_dist_extra_bits` | `D` (data, 32 B) | present `D` | `pub static mut cp_dist_extra_bits: [u8; 32]` |
| 8 | `cp_dist_base` | `D` (data, 128 B) | present `D` | `pub static mut cp_dist_base: [u32; 32]` |
| 9 | `cp_error_reason` | `B` (bss, ptr) | present `B` | `pub static mut cp_error_reason: *const c_char` |

**Symbol diff (C defined − Rust defined): EMPTY.** 0 missing symbols.

## Not exported by either side (`static` in C, private in Rust)

These are `static` in `c_src/src/lib.c`, therefore not part of the ABI, and are
correspondingly private in the Rust crate. They are exercised indirectly through
`cp_inflate`:

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_paeth`,
`cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter`.

`cp_paeth`/`cp_unfilter`/`cp_chunk`/`cp_find`/`cp_make32` are `static` **and**
never called from any non-`static` function in the C file, so they are dead in
both libraries (the C compiler keeps them out of the dynamic symbol table; the
Rust side keeps them behind `#[allow(dead_code)]`). No export wrapper is
required and adding one would *break* parity.

## Undefined (imported) symbols

C `.so` imports only libc: `__assert_fail`, `calloc`, `free`, `memcmp`,
`memcpy`, `memset` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).
The Rust `.so` imports only libc/`libgcc` equivalents. **0 missing/undefined
non-libc symbols on the Rust side.**

## Build-configuration note (affects Phase C)

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no `-DNDEBUG`
(`C_FLAGS = -fPIC` only), and the C `.so` imports `__assert_fail`.
**`assert()` is LIVE in the C library**: assert-tripping input `abort()`s the
process instead of returning.

This was measured, not assumed. Before the fix, a child-process differential
showed the C aborting (SIGABRT) on six inputs — including `in_bytes == 0` — where
the Rust silently returned. The Rust now transcribes all ten `assert()` calls as
`cp_assert!`, which writes a diagnostic and calls `process::abort()`
(`process::abort` rather than `panic!` so the behaviour is SIGABRT in every build
profile and never unwinds across the FFI boundary). Verification compares the
wait status **and the assertion expression that fired**, so "both crashed
somehow" is not accepted. See `ERRORS.md` rows 11-20 and
`tests/phase_c_aborts.rs`.

## Verification status

| gate | result |
|------|--------|
| C symbols exported by the Rust `.so` | 9 / 9 |
| symbols the Rust `.so` exports that the C does not | 0 |
| undefined non-libc symbols in the Rust `.so` | 0 |
| feature combinations checked | 1 (no `[features]` declared) |

Reproduce with `translation/scripts/verify_all.sh`, which rebuilds both
libraries, diffs `nm -D` in both directions, and runs every suite under every
feature combination.
