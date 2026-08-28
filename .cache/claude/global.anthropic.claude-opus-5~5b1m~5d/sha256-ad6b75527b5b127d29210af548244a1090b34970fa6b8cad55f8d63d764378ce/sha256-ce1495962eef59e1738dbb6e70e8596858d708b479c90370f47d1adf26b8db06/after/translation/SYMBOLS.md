# SYMBOLS.md — public symbol surface (Phase A)

Derived mechanically from

```
nm -D --defined-only c_src/build/libharvest-work-BA3r4V.so
nm -D --defined-only translation/target/release/libpinflate_lib.so
```

The C library is built by CMake with no `CMAKE_BUILD_TYPE`, i.e. `gcc -O0` and
**`NDEBUG` not defined** — the reference `.so` has an undefined reference to
`__assert_fail`, so every `assert()` in `c_src/src/lib.c` is live and is part of
the observable surface (see `ERRORS.md`).

## Defined (exported) symbols

| # | symbol | type | size | in C `.so` | in Rust `.so` | notes |
|---|--------|------|------|------------|---------------|-------|
| 1 | `pinflate`            | `T` (text) | 0x29b / 0x95c | yes | yes | the only function with external linkage (`c_src/include/lib.h`) |
| 2 | `cp_error_reason`     | `B` (bss)  | 8     | yes | yes | `const char *`, written by the 6 error paths |
| 3 | `cp_fixed_table`      | `D` (data) | 0x140 | yes | yes | `uint8_t[288+32]`, mutable, read by `cp_fixed` |
| 4 | `cp_permutation_order`| `D`        | 0x13  | yes | yes | `uint8_t[19]`, mutable, read by `cp_dynamic` |
| 5 | `cp_len_extra_bits`   | `D`        | 0x1f  | yes | yes | `uint8_t[29+2]`, mutable, read by `cp_block` |
| 6 | `cp_len_base`         | `D`        | 0x7c  | yes | yes | `uint32_t[29+2]`, mutable, read by `cp_block` |
| 7 | `cp_dist_extra_bits`  | `D`        | 0x20  | yes | yes | `uint8_t[30+2]`, mutable, read by `cp_block` |
| 8 | `cp_dist_base`        | `D`        | 0x80  | yes | yes | `uint32_t[30+2]`, mutable, read by `cp_block` |

**Symbol diff: EMPTY.** Both `.so`s export exactly the same 8 names, with
byte-identical sizes for all 7 data objects. Enforced by the automated test
`tests/symbols.rs::symbol_parity` (runs `nm -D` on both libraries and diffs the
name sets, and compares the sizes of the data objects).

The exported *sizes* matter because all seven globals are writable from outside
the library; `tests/differential.rs` mutates them through the `.so` exports and
requires identical behaviour from both implementations.

## `static` (internal-linkage) C functions — intentionally NOT exported

These have internal linkage in C and therefore must **not** appear in `nm -D`.
All are translated (private `fn`s in `src/lib.rs`) because `pinflate` needs them:

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`.

`cp_make_pixel_a` / `cp_make_pixel` (and the `cp_pixel_t` / `cp_image_t` types)
are dead code in the C translation unit; they are still translated so the file
is complete, and they are `static` so they add no symbols.

## Undefined symbols

C `.so`: `__assert_fail`, `calloc`, `free`, `memcpy`, `memset` (+ the usual weak
`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Rust `.so`: the same libc entry points (`__assert_fail`, `calloc`, `free`,
`memcpy`, `memset`) plus the Rust standard library's own libc/libgcc imports
(`_Unwind_*`, `abort`, `malloc`, `mmap64`, …). **0 missing/undefined non-libc
symbols** — every undefined symbol in the Rust `.so` is provided by
`libc.so.6`/`libgcc_s.so.1`, which is verified by the fact that both `.so`s
`dlopen()` successfully in the differential tests.

## Verification results

```
$ nm -D --defined-only c_src/build/libharvest-work-BA3r4V.so | awk '{print $NF}' | sort  \
  > /tmp/c.txt
$ nm -D --defined-only translation/target/release/libpinflate_lib.so | awk '{print $NF}' \
  | sort > /tmp/r.txt
$ diff /tmp/c.txt /tmp/r.txt      # -> no output, for both the debug and the
                                  #    release cdylib
```

* 8 / 8 symbols present, identical names, identical kinds (`T` / `D` / `B`),
  identical sizes for all seven data objects.
* **0 missing symbols, 0 unresolved non-libc symbols** — `tests/symbols.rs`
  additionally `dlopen`s both libraries with `RTLD_NOW`, which resolves every
  relocation up front and therefore fails if any import is unsatisfied.
* No Rust-only symbol shadows a C one: the Rust `.so` exports *exactly* the same
  8 names and nothing else (the `static` C helpers stay private in both).
* Nothing is stubbed: every symbol is backed by the translated implementation and
  is exercised by `tests/differential.rs` (see `CONFIGS.md` rows 53-60 for the
  seven data objects, which the tests mutate through the `.so` export and then
  compare behaviour).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one (`cargo test`, `cargo test --release`,
`cargo test --no-default-features`, `cargo test --all-features` are all the same
build). `run_verification.sh` enumerates the feature powerset from `Cargo.toml`
(which is empty) and additionally runs the whole suite in both `dev` and
`release` profiles, because `[profile.release] panic = "abort"` and the
optimiser are the only things that differ between configurations here.
