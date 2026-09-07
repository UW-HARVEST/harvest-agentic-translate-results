# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-NYpHHI.so
nm -D --defined-only translation/target/release/libsh_geti_lib.so
```

The C library is built from exactly one translation unit (`c_src/src/lib.c`,
an inlined copy of `stb_ds.h` plus the `strkey` / `sh_geti` driver code at the
bottom of the file). `c_src/include/lib.h` declares only `void sh_geti(int)`.
There is **no** second C module, so there is no un-translated C source.

## Exported (T) symbols

| # | C symbol | in C `.so` | in Rust `.so` | notes |
|---|----------|-----------|---------------|-------|
| 1 | `sh_geti` | yes | yes | driver entry point, only symbol in the public header |
| 2 | `stbds_arrfreef` | yes | yes | |
| 3 | `stbds_arrgrowf` | yes | yes | |
| 4 | `stbds_hash_bytes` | yes | yes | |
| 5 | `stbds_hash_string` | yes | yes | |
| 6 | `stbds_hmdel_key` | yes | yes | |
| 7 | `stbds_hmfree_func` | yes | yes | |
| 8 | `stbds_hmget_key` | yes | yes | |
| 9 | `stbds_hmget_key_ts` | yes | yes | |
| 10 | `stbds_hmput_default` | yes | yes | |
| 11 | `stbds_hmput_key` | yes | yes | |
| 12 | `stbds_rand_seed` | yes | yes | |
| 13 | `stbds_shmode_func` | yes | yes | |
| 14 | `stbds_stralloc` | yes | yes | |
| 15 | `stbds_strreset` | yes | yes | |
| 16 | `strkey` | yes | yes | |

## `static` C functions (deliberately NOT exported by either side)

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`. All are `static` in the C and private in the Rust; they are
exercised indirectly through the exported entry points.

Note: `stbds_unit_tests` is *declared* `extern` in `lib.c` but never defined,
so it is absent from the C `.so` too — correctly absent from Rust.
`buffer` (the `strkey` scratch buffer) is `static` in C, so it is not an
exported symbol; the Rust equivalent `BUFFER` is likewise private.

## Symbol diff

```
comm -3 <(nm -D --defined-only <c.so>  | awk '{print $3}' | sort) \
        <(nm -D --defined-only <rs.so> | awk '{print $3}' | sort | grep -v '^_')
```

Result: **empty**. 0 missing symbols.

## Undefined symbols in the Rust `.so`

All `U`/`w` entries are libc / libgcc-unwind / Rust-runtime imports
(`realloc`, `free`, `memmove`, `memcpy`, `bcmp`, `strcmp`, `strlen`,
`printf`, `sprintf`, `abort`, plus the std panic/backtrace machinery:
`_Unwind_*`, `dl_iterate_phdr`, `pthread_key_*`, `open64`, `read`, ...).
**0 undefined non-libc symbols.**
