# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Source of truth: `nm -D --defined-only` on

* C   : `c_src/build/libharvest-work-W0GFJp.so`
* Rust: `translation/target/release/libarr_ins_lib.so`

Regenerate / re-verify with `./check_symbols.sh`.

## Public (dynamic, defined) symbols exported by the C `.so`

| # | symbol | C signature (from `c_src/src/lib.c`) | in Rust `.so` |
|---|--------|--------------------------------------|---------------|
| 1 | `arr_ins` | `void arr_ins(int num)` | yes |
| 2 | `strkey` | `char *strkey(int n)` | yes |
| 3 | `stbds_rand_seed` | `void stbds_rand_seed(size_t seed)` | yes |
| 4 | `stbds_hash_bytes` | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | yes |
| 5 | `stbds_hash_string` | `size_t stbds_hash_string(char *str, size_t seed)` | yes |
| 6 | `stbds_arrgrowf` | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | yes |
| 7 | `stbds_arrfreef` | `void stbds_arrfreef(void *a)` | yes |
| 8 | `stbds_hmfree_func` | `void stbds_hmfree_func(void *a, size_t elemsize)` | yes |
| 9 | `stbds_hmget_key` | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes |
| 10 | `stbds_hmget_key_ts` | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | yes |
| 11 | `stbds_hmput_default` | `void *stbds_hmput_default(void *a, size_t elemsize)` | yes |
| 12 | `stbds_hmput_key` | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes |
| 13 | `stbds_hmdel_key` | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | yes |
| 14 | `stbds_shmode_func` | `void *stbds_shmode_func(size_t elemsize, int mode)` | yes |
| 15 | `stbds_stralloc` | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | yes |
| 16 | `stbds_strreset` | `void stbds_strreset(stbds_string_arena *a)` | yes |

`stbds_unit_tests` is only `extern`-declared in the C source and never defined,
so it is **not** in the C `.so` and must not be in the Rust `.so` either.

Symbols that are `static` in C (`stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`, `stbds_hash_seed`, `buffer`) are private in
both builds and are intentionally not exported.

## Result (verified by `./check_symbols.sh` and `tests/phase_d.rs::d_01..d_03`)

```
C   .so : c_src/build/libharvest-work-W0GFJp.so        (16 defined dynamic symbols)
Rust.so : translation/target/release/libarr_ins_lib.so (16 defined dynamic symbols)

comm -23 c_syms.txt rust_syms.txt   ->   (empty)
comm -13 c_syms.txt rust_syms.txt   ->   (empty)
```

* Symbols exported by the C `.so` but missing from the Rust `.so`: **0**
* Extra symbols exported by the Rust `.so`: **0**
* Undefined **non-libc** symbols in the Rust `.so`: **0**.
  `nm -D --undefined-only` lists only glibc and libgcc imports
  (`realloc`, `free`, `malloc`, `calloc`, `posix_memalign`, `memcpy`, `memmove`,
  `memset`, `bcmp`, `strlen`, `abort`, `write`, `writev`, `read`, `open64`,
  `mmap64`, `munmap`, `getenv`, `getcwd`, `readlink`, `realpath`, `syscall`,
  `gettid`, `pthread_*`, `dl_iterate_phdr`, `_Unwind_*`, `__cxa_*`,
  `__errno_location`, `_ITM_*`, `__gmon_start__`), all resolved by
  `libc.so.6` / `libgcc_s.so.1` — `ldd` reports no `not found`.

No module was skipped: every function defined in `c_src/src/lib.c` — including
the `static` ones (`stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`) and the `stbds_load_32_or_64` /
`STBDS_SIPROUND` / `stbds_arr*` macros — has a real translated counterpart in
`src/lib.rs`.  There are no stubs and no `unimplemented!()`/`todo!()` anywhere
in the crate.
