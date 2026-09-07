# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   `.so`: `c_src/build/libharvest-work-PYzwWM.so`
  (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `translation/target/release/libunfilter_lib.so`
  (`cargo build --release`)

## Defined dynamic symbols

| # | symbol | C kind | Rust kind | in C | in Rust | notes |
|---|--------|--------|-----------|------|---------|-------|
| 1 | `cp_dist_base`        | `D` (data)  | `D` | yes | yes | `uint32_t[30+2]` / `pub static mut [u32; 32]` |
| 2 | `cp_dist_extra_bits`  | `D` (data)  | `D` | yes | yes | `uint8_t[30+2]`  / `pub static mut [u8; 32]` |
| 3 | `cp_error_reason`     | `B` (bss)   | `B` | yes | yes | `const char *`, zero-initialised |
| 4 | `cp_fixed_table`      | `D` (data)  | `D` | yes | yes | `uint8_t[288+32]` |
| 5 | `cp_inflate`          | `T` (text)  | `T` | yes | yes | `int cp_inflate(void*,int,void*,int)` |
| 6 | `cp_len_base`         | `D` (data)  | `D` | yes | yes | `uint32_t[29+2]` |
| 7 | `cp_len_extra_bits`   | `D` (data)  | `D` | yes | yes | `uint8_t[29+2]` |
| 8 | `cp_permutation_order`| `D` (data)  | `D` | yes | yes | `uint8_t[19]` |
| 9 | `unfilter`            | `T` (text)  | `T` | yes | yes | `int unfilter(int,int,int,uint8_t*)` — the only symbol in `include/lib.h` |

**Symbol diff (C \ Rust): EMPTY.** **Symbol diff (Rust \ C): EMPTY.**

`tests/symbols.rs` re-derives both lists with `nm -D` at test time and asserts
the diff is empty, so this table cannot silently rot.

## `static` (non-exported) C functions

These are `static` in `lib.c` and therefore *not* part of the dynamic symbol
table.  They are all translated (as private Rust `unsafe fn`s) and are covered
transitively through `cp_inflate` / `unfilter`:

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_paeth`,
`cp_make32`, `cp_chunk`, `cp_find`.

`cp_make_pixel*`, `cp_make32`, `cp_chunk` and `cp_find` are dead code in the C
translation unit as well (nothing in `lib.c` calls them); they are translated
for completeness but are unreachable from either `.so`'s public surface.

## Undefined (imported) symbols

The C `.so` imports `calloc`, `free`, `memcmp`, `memcpy`, `memset` and — when
built *without* `NDEBUG` (which is what the plain `cmake ..` configure does) —
`__assert_fail`.  The Rust `.so` imports the equivalent libc/`std` primitives.
There are **0 missing/undefined non-libc symbols** in the Rust `.so`
(`nm -D -u` shows only glibc entries plus the usual weak
`_ITM_*`/`__gmon_start__`/`__cxa_finalize` markers).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table at all**, so the only
build configuration is the default one (`--no-default-features` and
`--all-features` are equivalent to the default).  Phase D's "repeat B–C for
every feature combo" therefore collapses to a single combination; the test
runner script still loops over `{default, --no-default-features,
--all-features}` to prove it.
