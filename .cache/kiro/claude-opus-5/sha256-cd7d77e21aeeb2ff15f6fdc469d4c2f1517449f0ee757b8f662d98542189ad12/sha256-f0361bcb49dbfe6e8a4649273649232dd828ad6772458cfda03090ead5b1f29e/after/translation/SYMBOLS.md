# SYMBOLS.md — exported-symbol parity

Source of truth: `nm -D --defined-only` on

* C:    `c_src/build/libharvest-work-po7J5v.so`
* Rust: `translation/target/release/libhm_geti_lib.so`

The C translation unit is `stb_ds.h`'s `STB_DS_IMPLEMENTATION` body plus the
small driver at the bottom of `lib.c` (`strkey`, `hm_geti`). Only
`void hm_geti(int num);` is declared in `include/lib.h`; every other symbol
below is a non-`static` definition in `src/lib.c` and therefore part of the
`.so` ABI surface. All 16 must be verified.

## Table

| # | symbol | C signature (from `src/lib.c`) | in C `.so` | in Rust `.so` | Rust impl site |
|---|--------|-------------------------------|-----------|---------------|----------------|
| 1 | `stbds_arrgrowf` | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | T | T | `src/arr.rs` |
| 2 | `stbds_arrfreef` | `void stbds_arrfreef(void *a)` | T | T | `src/arr.rs` |
| 3 | `stbds_rand_seed` | `void stbds_rand_seed(size_t seed)` | T | T | `src/hash.rs` |
| 4 | `stbds_hash_string` | `size_t stbds_hash_string(char *str, size_t seed)` | T | T | `src/hash.rs` |
| 5 | `stbds_hash_bytes` | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | T | T | `src/hash.rs` |
| 6 | `stbds_hmfree_func` | `void stbds_hmfree_func(void *a, size_t elemsize)` | T | T | `src/hashmap.rs` |
| 7 | `stbds_hmget_key_ts` | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | T | T | `src/hashmap.rs` |
| 8 | `stbds_hmget_key` | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | T | T | `src/hashmap.rs` |
| 9 | `stbds_hmput_default` | `void *stbds_hmput_default(void *a, size_t elemsize)` | T | T | `src/hashmap.rs` |
| 10 | `stbds_hmput_key` | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | T | T | `src/hashmap.rs` |
| 11 | `stbds_shmode_func` | `void *stbds_shmode_func(size_t elemsize, int mode)` | T | T | `src/hashmap.rs` |
| 12 | `stbds_hmdel_key` | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | T | T | `src/hashmap.rs` |
| 13 | `stbds_stralloc` | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | T | T | `src/strings.rs` |
| 14 | `stbds_strreset` | `void stbds_strreset(stbds_string_arena *a)` | T | T | `src/strings.rs` |
| 15 | `strkey` | `char *strkey(int n)` | T | T | `src/unit_tests.rs` |
| 16 | `hm_geti` | `void hm_geti(int num)` | T | T | `src/unit_tests.rs` |

## `static` C functions (no symbol, exercised only indirectly)

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`. All are present in the Rust crate as private functions and are
covered transitively by the differential tests (the hash-index internals are
compared field-by-field and bucket-by-bucket, so `stbds_make_hash_index`,
`stbds_log2` and `stbds_probe_position` are checked directly).

`extern` declarations in `lib.c` with no definition in this TU
(`stbds_unit_tests`) produce no symbol in either `.so` — nothing to translate.

## Verification result

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-po7J5v.so    | awk '{print $2,$3}' | sort) \
       <(nm -D --defined-only translation/target/release/libhm_geti_lib.so | awk '{print $2,$3}' | sort)
<empty>
```

* Symbols in C `.so` but not Rust `.so`: **0**
* Symbols in Rust `.so` but not C `.so`: **0**
* Undefined symbols in Rust `.so`: only libc / libgcc_s runtime imports
  (`malloc`, `realloc`, `free`, `memcpy`, `memmove`, `memset`, `bcmp`,
  `strcmp`, `strlen`, `abort`, `__errno_location`, `_Unwind_*`, `__cxa_*`,
  pthread TLS shims, and the std backtrace syscalls). **0** non-libc
  undefined symbols.

## Struct-layout parity (needed by the differential harness)

Confirmed with a standalone C probe using the definitions copied from
`src/lib.c` and with `core::mem::size_of` on the Rust side:

| type | size | notable offsets |
|------|------|-----------------|
| `stbds_array_header` | 32 | length 0, capacity 8, hash_table 16, temp 24 |
| `stbds_string_block` | 16 | next 0, storage 8 |
| `stbds_string_arena` | 24 | storage 0, remaining 8, block 16, mode 17 |
| `stbds_hash_bucket` | 128 | hash[8] 0, index[8] 64 |
| `stbds_hash_index` | 104 | seed 56, slot_count_log2 64, string 72, storage 96 |
