# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-xaK3up.so | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort
nm -D --defined-only translation/target/release/libarr_del_lib.so | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort
```

The C library is a single translation unit (`c_src/src/lib.c`) that inlines the
whole of `stb_ds.h`'s implementation plus the `strkey` / `arr_del` test helpers.
The public header (`c_src/include/lib.h`) only declares `arr_del`, but the `.so`
exports every non-`static` definition, so all of those are part of the surface.

## Exported symbol table

| # | symbol | C `.so` | Rust `.so` | signature |
|---|--------|---------|------------|-----------|
| 1 | `arr_del` | T | T | `void arr_del(int num)` |
| 2 | `stbds_arrfreef` | T | T | `void stbds_arrfreef(void *a)` |
| 3 | `stbds_arrgrowf` | T | T | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` |
| 4 | `stbds_hash_bytes` | T | T | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` |
| 5 | `stbds_hash_string` | T | T | `size_t stbds_hash_string(char *str, size_t seed)` |
| 6 | `stbds_hmdel_key` | T | T | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` |
| 7 | `stbds_hmfree_func` | T | T | `void stbds_hmfree_func(void *p, size_t elemsize)` |
| 8 | `stbds_hmget_key` | T | T | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` |
| 9 | `stbds_hmget_key_ts` | T | T | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` |
| 10 | `stbds_hmput_default` | T | T | `void *stbds_hmput_default(void *a, size_t elemsize)` |
| 11 | `stbds_hmput_key` | T | T | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` |
| 12 | `stbds_rand_seed` | T | T | `void stbds_rand_seed(size_t seed)` |
| 13 | `stbds_shmode_func` | T | T | `void *stbds_shmode_func(size_t elemsize, int mode)` |
| 14 | `stbds_stralloc` | T | T | `char *stbds_stralloc(stbds_string_arena *a, char *str)` |
| 15 | `stbds_strreset` | T | T | `void stbds_strreset(stbds_string_arena *a)` |
| 16 | `strkey` | T | T | `char *strkey(int n)` |

**Missing from Rust `.so`: NONE.** Symbol diff (`comm -23`) is empty in both
directions for `T`/`B`/`D` symbols.

## Intentionally NOT exported (static in C, private in Rust)

These are `static` in `c_src/src/lib.c` and therefore absent from `nm -D`; the
Rust translation keeps them private too, so the two `.so`s stay in parity:

`buffer`, `stbds_hash_seed`, `stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`.

`stbds_unit_tests` is `extern`-declared in the C but never defined, so it is
not exported by either library.

## Undefined (imported) symbols

The Rust `.so` imports only libc/runtime symbols. Non-libc undefined symbol
count: **0**. Verified with:

```
nm -D -u translation/target/release/libarr_del_lib.so
```
