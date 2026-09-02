# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-VD0qUB.so   | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libarr_del_lib.so | awk '{print $3}' | sort > /tmp/r_syms.txt
diff /tmp/c_syms.txt /tmp/r_syms.txt          # -> empty
```

The C library is a single translation unit (`c_src/src/lib.c`, an inlined copy of
`stb_ds.h` plus the `strkey` / `arr_del` test helpers).  The public header
(`c_src/include/lib.h`) declares only `arr_del`, but every non-`static`
definition in `lib.c` lands in the `.so`'s dynamic symbol table, so the real
surface is the 16 symbols below.

| # | symbol | C signature (from `lib.c`) | in C `.so` | in Rust `.so` | notes |
|---|--------|----------------------------|:---:|:---:|-------|
| 1 | `arr_del`             | `void arr_del(int num)` | yes | yes | the only symbol in `include/lib.h` |
| 2 | `strkey`              | `char *strkey(int n)` | yes | yes | writes into the module-static `buffer[256]` |
| 3 | `stbds_rand_seed`     | `void stbds_rand_seed(size_t seed)` | yes | yes | sets the module-static `stbds_hash_seed` |
| 4 | `stbds_hash_bytes`    | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | yes | yes | thin wrapper over `static stbds_siphash_bytes` |
| 5 | `stbds_hash_string`   | `size_t stbds_hash_string(char *str, size_t seed)` | yes | yes | |
| 6 | `stbds_arrgrowf`      | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | yes | yes | |
| 7 | `stbds_arrfreef`      | `void stbds_arrfreef(void *a)` | yes | yes | unconditional `free(header(a))` — no null check |
| 8 | `stbds_hmfree_func`   | `void stbds_hmfree_func(void *p, size_t elemsize)` | yes | yes | |
| 9 | `stbds_hmget_key`     | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes | |
|10 | `stbds_hmget_key_ts`  | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | yes | yes | |
|11 | `stbds_hmput_default` | `void *stbds_hmput_default(void *a, size_t elemsize)` | yes | yes | |
|12 | `stbds_hmput_key`     | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes | |
|13 | `stbds_hmdel_key`     | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | yes | yes | |
|14 | `stbds_shmode_func`   | `void *stbds_shmode_func(size_t elemsize, int mode)` | yes | yes | |
|15 | `stbds_stralloc`      | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | yes | yes | |
|16 | `stbds_strreset`      | `void stbds_strreset(stbds_string_arena *a)` | yes | yes | |

## Symbols declared in `lib.c` but NOT defined there

These appear as `extern` prototypes only; they are *not* in the C `.so`'s
defined-symbol list, so the Rust `.so` must not export them either:

* `stbds_unit_tests` — declared `extern void stbds_unit_tests(void);`, never defined.

## `static` (internal) C functions — deliberately not exported

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`.  These are `static` in C, so they have no dynamic symbol; the
Rust translation keeps them as private `fn`s.  They are still covered
indirectly by every test that drives the public entry points.

## Result

`diff` of the two sorted symbol lists is **empty** — 16 / 16 match, 0 missing.

Undefined symbols in the Rust `.so` are libc / `libgcc` unwinder only
(`realloc`, `free`, `memset`, `memcpy`, `memmove`, `bcmp`, `strlen`, `strcmp`,
`sprintf`, plus the Rust std panic-runtime/`_Unwind_*` set).  No non-libc
undefined symbols.
