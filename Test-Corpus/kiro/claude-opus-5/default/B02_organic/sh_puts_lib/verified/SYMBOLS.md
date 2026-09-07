# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

Sources:

```
nm -D --defined-only c_src/build/libharvest-work-3DSWrw.so
nm -D --defined-only translation/target/release/libsh_puts_lib.so
```

The C library is built from a single translation unit (`c_src/src/lib.c`, an
inlined copy of `stb_ds.h` plus the `sh_puts` driver). Everything that is not
`static` becomes a dynamic symbol; there is no second module and no
macro-generated symbol (all `stbds_*` map/array operations are *macros* that
expand at the call site into calls to the 16 exported helpers below).

## Exported symbol table

| # | symbol | C signature | in C `.so` | in Rust `.so` | notes |
|---|--------|-------------|-----------|--------------|-------|
| 1 | `stbds_arrgrowf` | `void *(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | ✅ | ✅ | array realloc core |
| 2 | `stbds_arrfreef` | `void (void *a)` | ✅ | ✅ | frees `stbds_header(a)` |
| 3 | `stbds_rand_seed` | `void (size_t seed)` | ✅ | ✅ | sets file-static `stbds_hash_seed` |
| 4 | `stbds_hash_string` | `size_t (char *str, size_t seed)` | ✅ | ✅ | rotate/mix string hash |
| 5 | `stbds_hash_bytes` | `size_t (void *p, size_t len, size_t seed)` | ✅ | ✅ | wraps `stbds_siphash_bytes` |
| 6 | `stbds_hmfree_func` | `void (void *a, size_t elemsize)` | ✅ | ✅ | frees table + strdup'd keys |
| 7 | `stbds_hmget_key_ts` | `void *(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | ✅ | ✅ | lookup, index via `*temp` |
| 8 | `stbds_hmget_key` | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | ✅ | ✅ | lookup, index via header `temp` |
| 9 | `stbds_hmput_default` | `void *(void *a, size_t elemsize)` | ✅ | ✅ | allocates slot `[-1]` |
| 10 | `stbds_hmput_key` | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | ✅ | ✅ | insert/find, grows table |
| 11 | `stbds_shmode_func` | `void *(size_t elemsize, int mode)` | ✅ | ✅ | `sh_new_arena` / `sh_new_strdup` |
| 12 | `stbds_hmdel_key` | `void *(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | ✅ | ✅ | tombstone + swap-with-last |
| 13 | `stbds_stralloc` | `char *(stbds_string_arena *a, char *str)` | ✅ | ✅ | arena string copy |
| 14 | `stbds_strreset` | `void (stbds_string_arena *a)` | ✅ | ✅ | frees arena block list |
| 15 | `strkey` | `char *(int n)` | ✅ | ✅ | `sprintf(buffer,"test_%d",n)` |
| 16 | `sh_puts` | `void (int num)` | ✅ | ✅ | the public entry point in `lib.h` |

## `static` (deliberately NOT exported by either library)

`stbds_hash_seed`, `buffer`, `stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`.

`stbds_unit_tests` is `extern`-declared in the C source but never defined and
therefore appears in neither `.so`. The Rust translation likewise omits it.

## Verification

```
$ comm -23 <(nm -D --defined-only c_src/build/libharvest-work-3DSWrw.so   | awk '{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/libsh_puts_lib.so | awk '{print $3}' | sort)
(empty)
```

- [x] 0 C symbols missing from the Rust `.so`.
- [x] Rust `.so` has no undefined non-libc symbols
      (`nm -D -u` → only `realloc/free/strlen/strcmp/memcmp/printf` +
      `memcpy/memmove/memset` + `__assert_fail`'s `abort` + Rust-std libc/loader
      imports and glibc/GCC weak symbols).

Re-run at any time with `translation/verify.sh`, which rebuilds both libraries,
diffs `nm -D`, enumerates every Cargo feature combination, and runs the full
differential suite in both the debug and release profiles.
