# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  : `c_src/build/libharvest-work-FtZjFD.so`
* Rust: `translation/target/release/libpinflate_lib.so`

## Defined (exported) symbols in the C `.so`

| # | symbol | C type / kind | size | present in Rust `.so`? | notes |
|---|--------|---------------|------|------------------------|-------|
| 1 | `pinflate`            | `T` function | — | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn pinflate` |
| 2 | `cp_fixed_table`      | `D` `uint8_t[320]`  | 320 | YES | `pub static mut cp_fixed_table: [u8; 320]` |
| 3 | `cp_permutation_order`| `D` `uint8_t[19]`   | 19  | YES | `pub static mut cp_permutation_order: [u8; 19]` |
| 4 | `cp_len_extra_bits`   | `D` `uint8_t[31]`   | 31  | YES | `pub static mut cp_len_extra_bits: [u8; 31]` |
| 5 | `cp_len_base`         | `D` `uint32_t[31]`  | 124 | YES | `pub static mut cp_len_base: [u32; 31]` |
| 6 | `cp_dist_extra_bits`  | `D` `uint8_t[32]`   | 32  | YES | `pub static mut cp_dist_extra_bits: [u8; 32]` |
| 7 | `cp_dist_base`        | `D` `uint32_t[32]`  | 128 | YES | `pub static mut cp_dist_base: [u32; 32]` |
| 8 | `cp_error_reason`     | `B` `const char *`  | 8   | YES | `pub static mut cp_error_reason: *const c_char` |

`static` C functions (`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`,
`cp_ptr`, `cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`,
`cp_build`, `cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`)
have internal linkage and export no dynamic symbols; they are translated as
private Rust `fn`s. No export wrapper is required or permitted for them.

## Undefined symbols

C `.so` imports only libc (`__assert_fail`, `calloc`, `free`, `memcpy`,
`memset`) plus the four standard weak glibc/ITM stubs.

Rust `.so` imports only libc / `libgcc_s` unwinder symbols. No non-libc
undefined symbols in either object.

## Diff result

```
$ nm -D --defined-only <c.so>    | awk '{print $3}' | sort > /tmp/c.syms
$ nm -D --defined-only <rust.so> | awk '{print $3}' | sort > /tmp/r.syms
$ comm -23 /tmp/c.syms /tmp/r.syms      # in C, missing from Rust
(empty)
```

**Status: 0 missing symbols. 0 undefined non-libc symbols in Rust.** ✅

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configurations are the default one and `--no-default-features` (identical here).
This is established mechanically, not by inspection:
`scripts/feature_matrix.sh` extracts the feature list from `Cargo.toml`, forms
the powerset, and for each combination runs `cargo check`, `cargo build`,
`scripts/symbol_parity.sh` and the full differential suite. Result:

```
features declared in Cargo.toml: 0 ()
combinations to verify: 2          # DEFAULT and NONE
FEATURE MATRIX: OK
```

If a `[features]` section is ever added, the script picks it up automatically.

## Phase D result

| gate | status |
|---|---|
| `nm -D`: 0 symbols missing from the Rust `.so` | ✅ `scripts/symbol_parity.sh` |
| `nm -D`: 0 undefined non-libc symbols in either `.so` | ✅ |
| every `CONFIGS.md` row passes over randomised inputs | ✅ 49 tests |
| every `ERRORS.md` row tested or proven unreachable | ✅ 22 tests |
| binary/driver stdout comparison | n/a — neither project builds an executable (`add_library(... SHARED)` / `crate-type = ["cdylib"]`) |
| all of the above under every feature combination | ✅ 2 of 2 |
| all of the above in both `debug` and `release` profiles | ✅ (the release profile exposed two real divergences, see `src/lib.rs`'s notes on `cp_decode`) |

