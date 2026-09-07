# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on
`c_src/build/libharvest-work-XY2vLq.so` (C) vs
`translation/target/release/libstr_put_lib.so` (Rust).

The C translation unit is a single file (`c_src/src/lib.c`) that inlines a copy of
`stb_ds.h` plus the `strkey` / `str_put` driver. Everything declared `static`
in C (`stbds_hash_seed`, `buffer`, `stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`) is intentionally NOT exported by either
library. `stbds_unit_tests` is declared `extern` but never defined, so it is not
exported by the C `.so` either.

## Exported symbol table (16 symbols)

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `stbds_arrgrowf`      | T | T | array growth allocator |
| 2 | `stbds_arrfreef`      | T | T | array free |
| 3 | `stbds_rand_seed`     | T | T | sets the file-static hash seed |
| 4 | `stbds_hash_string`   | T | T | rotate/mix string hash |
| 5 | `stbds_hash_bytes`    | T | T | wrapper over static `stbds_siphash_bytes` |
| 6 | `stbds_hmfree_func`   | T | T | frees map + strdup'd keys + arena |
| 7 | `stbds_hmget_key_ts`  | T | T | lookup, index via out-param |
| 8 | `stbds_hmget_key`     | T | T | lookup, index via header `temp` |
| 9 | `stbds_hmput_default` | T | T | ensures slot -1 exists |
| 10 | `stbds_hmput_key`    | T | T | insert/update, grows + rehashes |
| 11 | `stbds_shmode_func`  | T | T | creates a string-mode map |
| 12 | `stbds_hmdel_key`    | T | T | delete, tombstone, shrink/rebuild |
| 13 | `stbds_stralloc`     | T | T | string arena bump allocator |
| 14 | `stbds_strreset`     | T | T | frees arena block list |
| 15 | `strkey`             | T | T | `sprintf(buffer,"test_%d",n)` |
| 16 | `str_put`            | T | T | driver from `lib.h` |

## Diff result

```
$ diff <(nm -D --defined-only C.so  | awk '{print $3}' | sort) \
       <(nm -D --defined-only RS.so | awk '{print $3}' | sort)
(empty)
```

**0 missing symbols. 0 extra symbols. 0 undefined non-libc symbols in the Rust
`.so`** (Rust's undefined list is exactly the libc set the C also imports:
`realloc`, `free`, `memset`, `memcpy`, `memmove`, `memcmp`, `strcmp`, `strlen`,
`printf`, `sprintf`, plus the usual `__*` runtime hooks).

No module of the C source was skipped by the translation: `lib.c` is fully
covered by `translation/src/lib.rs`, including the file-static helpers, which are
private in both.

## How the parity is enforced by the tests

`tests/common/mod.rs` resolves **all 16 symbols by name** with
`libloading::Library::get` on *both* `.so` files and panics with
`missing symbol <name>` if either lookup fails. Every one of the 95 differential
tests therefore re-proves the symbol parity at run time, through the real
`#[no_mangle]`/`extern "C"` export wrappers — no Rust function is ever called
directly from the test crate (the crate is `crate-type = ["cdylib"]` only, so
linking against it is not even possible).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
build configuration is the default one. `run_verification.sh` enumerates the
powerset of the declared features (falling back to `default` plus
`--no-default-features`), rebuilds the `.so`, re-runs the `nm -D` diff and the
whole test suite for each; both configurations report
`0 missing, 0 extra (16 symbols)` and `0` non-libc undefined symbols.

```
=== 4. [default features] symbol parity (nm -D) ===
    0 missing, 0 extra (16 symbols)
    undefined non-libc symbols in the Rust .so:
      (none)
=== 4. [--no-default-features] symbol parity (nm -D) ===
    0 missing, 0 extra (16 symbols)
    undefined non-libc symbols in the Rust .so:
      (none)
```
