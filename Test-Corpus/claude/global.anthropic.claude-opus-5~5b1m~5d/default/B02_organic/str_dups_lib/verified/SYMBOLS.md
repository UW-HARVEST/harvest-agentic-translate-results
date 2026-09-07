# SYMBOLS.md — exported-symbol parity

C library: `c_src/build/libharvest-work-mQ2Rbe.so`
Rust library: `translation/target/release/libstr_dups_lib.so`

Generated with:

```sh
nm -D --defined-only <so> | awk '{print $3}' | sort
```

| # | symbol (C `.so`, `nm -D`) | present in Rust `.so` | notes |
|---|---------------------------|-----------------------|-------|
| 1 | `stbds_arrgrowf`      | yes | `#[no_mangle] extern "C"` |
| 2 | `stbds_arrfreef`      | yes | `#[no_mangle] extern "C"` |
| 3 | `stbds_rand_seed`     | yes | mutates global `stbds_hash_seed` |
| 4 | `stbds_hash_string`   | yes | |
| 5 | `stbds_hash_bytes`    | yes | wraps private `stbds_siphash_bytes` |
| 6 | `stbds_hmfree_func`   | yes | |
| 7 | `stbds_hmget_key_ts`  | yes | |
| 8 | `stbds_hmget_key`     | yes | |
| 9 | `stbds_hmput_default` | yes | |
| 10 | `stbds_hmput_key`    | yes | |
| 11 | `stbds_shmode_func`  | yes | |
| 12 | `stbds_hmdel_key`    | yes | |
| 13 | `stbds_stralloc`     | yes | |
| 14 | `stbds_strreset`     | yes | |
| 15 | `strkey`             | yes | uses private 256-byte static buffer |
| 16 | `str_dups`           | yes | public entry point from `include/lib.h` |

## Symbols intentionally NOT exported

These are `static` in the C translation unit and therefore absent from
`nm -D` on the C `.so`; they are private (non-`pub`, no `#[no_mangle]`) in Rust
too, so parity holds:

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`, `buffer`, `stbds_hash_seed`.

`stbds_unit_tests` is only `extern`-declared in `lib.c` (line 83) and never
defined, so it exists in neither `.so`. Nothing to translate.

## Result

```
$ comm -3 c_syms.txt r_syms.txt
(empty)
```

**0 missing symbols, 0 extra symbols.** Undefined symbols in the Rust `.so`
are libc only (`realloc`, `free`, `mem*`, `str*`, `printf`, `sprintf`,
`__errno_location`, unwinding/`std` internals).
