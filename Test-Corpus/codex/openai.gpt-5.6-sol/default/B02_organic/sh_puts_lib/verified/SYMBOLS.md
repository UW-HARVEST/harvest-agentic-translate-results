# Dynamic Symbol Surface

Source of truth:

```text
nm -D --defined-only ../c_src/build/libharvest-work-YuvvZb.so
```

The C shared object exports 16 global text symbols. The Rust column was
checked with `nm -D --defined-only target/release/libsh_puts_lib.so`.

| # | C symbol | C declaration/definition | Rust export |
|---|----------|--------------------------|-------------|
| 1 | `sh_puts` | `void sh_puts(int num)` | [x] |
| 2 | `stbds_arrfreef` | `void stbds_arrfreef(void *a)` | [x] |
| 3 | `stbds_arrgrowf` | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | [x] |
| 4 | `stbds_hash_bytes` | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | [x] |
| 5 | `stbds_hash_string` | `size_t stbds_hash_string(char *str, size_t seed)` | [x] |
| 6 | `stbds_hmdel_key` | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | [x] |
| 7 | `stbds_hmfree_func` | `void stbds_hmfree_func(void *p, size_t elemsize)` | [x] |
| 8 | `stbds_hmget_key` | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | [x] |
| 9 | `stbds_hmget_key_ts` | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | [x] |
| 10 | `stbds_hmput_default` | `void *stbds_hmput_default(void *a, size_t elemsize)` | [x] |
| 11 | `stbds_hmput_key` | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | [x] |
| 12 | `stbds_rand_seed` | `void stbds_rand_seed(size_t seed)` | [x] |
| 13 | `stbds_shmode_func` | `void *stbds_shmode_func(size_t elemsize, int mode)` | [x] |
| 14 | `stbds_stralloc` | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | [x] |
| 15 | `stbds_strreset` | `void stbds_strreset(stbds_string_arena *a)` | [x] |
| 16 | `strkey` | `char *strkey(int n)` | [x] |

The C object has only libc/toolchain undefined symbols (`assert`, allocation,
memory, string, stdio, and ELF runtime hooks). It has no undefined project
symbols. The defined-symbol set difference, C minus Rust, is empty.
