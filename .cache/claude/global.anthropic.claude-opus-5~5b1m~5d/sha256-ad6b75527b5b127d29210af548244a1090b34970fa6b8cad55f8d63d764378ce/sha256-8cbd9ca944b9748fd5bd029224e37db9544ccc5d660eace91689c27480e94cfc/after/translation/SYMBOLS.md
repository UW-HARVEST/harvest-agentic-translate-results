# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libharvest-work-3rdGDT.so` (built by CMake, no `CMAKE_BUILD_TYPE`,
i.e. `-O0` **with `assert()` live** — the `.so` has an undefined reference to
`__assert_fail`).

Rust shared library: `translation/target/release/libpinflate_lib.so`
(`crate-type = ["cdylib"]`).

Regenerate / diff with:

```sh
./check_symbols.sh
```

**Status: the symbol diff (C → Rust) is EMPTY.**  8 symbols exported by the C
`.so`, all 8 exported by the Rust `.so` under the same names, all 8 resolvable
through `dlsym`, all data symbols the same size.  Verified for the release
cdylib, for `--no-default-features`, and for the debug cdylib
(`./check_features.sh`).  Enforced by `tests/symbols.rs` (4 tests) as well as by
`check_symbols.sh`.

## Exported (defined) symbols

| # | symbol | C type | Rust definition | present in Rust `.so` |
|---|--------|--------|-----------------|-----------------------|
| 1 | `pinflate`             | `T` (text)  | `#[no_mangle] pub unsafe extern "C" fn pinflate` | yes |
| 2 | `cp_error_reason`      | `B` (bss)   | `#[no_mangle] pub static mut cp_error_reason: *const c_char` | yes |
| 3 | `cp_fixed_table`       | `D` (data)  | `#[no_mangle] pub static mut cp_fixed_table: [u8; 320]` | yes |
| 4 | `cp_permutation_order` | `D` (data)  | `#[no_mangle] pub static mut cp_permutation_order: [u8; 19]` | yes |
| 5 | `cp_len_extra_bits`    | `D` (data)  | `#[no_mangle] pub static mut cp_len_extra_bits: [u8; 31]` | yes |
| 6 | `cp_len_base`          | `D` (data)  | `#[no_mangle] pub static mut cp_len_base: [u32; 31]` | yes |
| 7 | `cp_dist_extra_bits`   | `D` (data)  | `#[no_mangle] pub static mut cp_dist_extra_bits: [u8; 32]` | yes |
| 8 | `cp_dist_base`         | `D` (data)  | `#[no_mangle] pub static mut cp_dist_base: [u32; 32]` | yes |

**Missing from Rust `.so`: none.** The symbol diff is empty (verified by
`check_symbols.sh`, which is also asserted by the integration test
`symbols::c_and_rust_export_the_same_symbols`).

Symbol *addresses* differ between the two libraries (link order is not part of
the ABI); the tests therefore compare symbol **contents**, not addresses.
The one place where this is observable is an out-of-bounds read past the end of
one of the tables — see the note at the bottom of `ERRORS.md`.

## Internal (`static`) C functions — translated, deliberately **not** exported

These have internal linkage in C and so must *not* appear in `nm -D`.  All of
them exist in the Rust translation as private `fn`s:

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`.

Types `cp_pixel_t`, `cp_image_t`, `cp_state_t` are translated as `#[repr(C)]`
structs.  `cp_state_t`'s exact layout is *observable*: `cp_decode` reads
`tree[-1]`, which aliases the struct member preceding `lit`/`dst`/`len`.  The
Rust translation static-asserts every field offset and `size_of` against the
values gcc produces (`size_of == 2464`).

## Undefined (imported) symbols

The C `.so` imports `__assert_fail`, `calloc`, `free`, `memcpy`, `memset`
(plus the usual weak `__gmon_start__`, `__cxa_finalize`,
`_ITM_*TMCloneTable`).  The Rust `.so` imports `__assert_fail`, `calloc`,
`free` and open-codes `memcpy`/`memset` through `core::ptr` (compiler builtins),
which is behaviourally identical.  There are **0 missing / unresolvable
non-libc undefined symbols** in the Rust `.so`.
