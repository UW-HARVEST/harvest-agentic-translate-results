# SYMBOLS.md — exported-symbol parity

Source of truth: `nm -D --defined-only` on both shared objects.

* C   : `c_src/build/libharvest-work-968HKH.so`
* Rust: `translation/target/release/libarr_ins_lib.so`

The C translation unit is a single file (`c_src/src/lib.c`, an inlined copy of
`stb_ds.h` plus the two test helpers `strkey` / `arr_ins`).  Everything else in
that file is `static` (`stbds_hash_seed`, `stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`, `buffer`) and therefore is *not* part of
the dynamic symbol table; the Rust side keeps those private too.

`stbds_unit_tests` is only `extern`-declared in the C source, never defined, so
it is undefined in the C `.so` as well and is not required from Rust.

## Symbol table

| # | C symbol (`nm -D`) | type | exported by Rust `.so` | notes |
|---|--------------------|------|------------------------|-------|
| 1 | `arr_ins`             | T | yes | `#[no_mangle] extern "C" fn arr_ins(c_int)` |
| 2 | `stbds_arrfreef`      | T | yes | |
| 3 | `stbds_arrgrowf`      | T | yes | |
| 4 | `stbds_hash_bytes`    | T | yes | |
| 5 | `stbds_hash_string`   | T | yes | |
| 6 | `stbds_hmdel_key`     | T | yes | |
| 7 | `stbds_hmfree_func`   | T | yes | |
| 8 | `stbds_hmget_key`     | T | yes | |
| 9 | `stbds_hmget_key_ts`  | T | yes | |
| 10 | `stbds_hmput_default`| T | yes | |
| 11 | `stbds_hmput_key`    | T | yes | |
| 12 | `stbds_rand_seed`    | T | yes | |
| 13 | `stbds_shmode_func`  | T | yes | |
| 14 | `stbds_stralloc`     | T | yes | |
| 15 | `stbds_strreset`     | T | yes | |
| 16 | `strkey`             | T | yes | |

## Result

```
comm -23 <c syms> <rust syms>   ->   (empty)
```

0 missing symbols.  Undefined (imported) symbols on the Rust side are libc only
(`realloc`, `free`, `memmove`, `memcpy`, `memcmp`, `strcmp`, `strlen` +
`__rust_no_alloc_shim_is_unstable`-style runtime bits), i.e. no non-libc
undefined symbols.

Verified by `tests/symbols.rs`, which shells out to `nm -D` on both objects and
asserts the difference is empty.
