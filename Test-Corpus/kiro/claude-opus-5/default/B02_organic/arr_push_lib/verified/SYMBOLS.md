# SYMBOLS.md — exported symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-Ut47mz.so   | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libarr_push_lib.so | awk '{print $3}' | sort > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing in Rust
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # extra in Rust
```

## Dynamic symbol table (C `.so` = ground truth)

| # | symbol | C signature (from `c_src/src/lib.c`) | in Rust `.so`? | Rust item |
|---|--------|--------------------------------------|----------------|-----------|
| 1 | `arr_push` | `void arr_push(int num)` | YES | `arr_push` |
| 2 | `stbds_arrfreef` | `void stbds_arrfreef(void *a)` | YES | `stbds_arrfreef` |
| 3 | `stbds_arrgrowf` | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | YES | `stbds_arrgrowf` |
| 4 | `stbds_hash_bytes` | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | YES | `stbds_hash_bytes` |
| 5 | `stbds_hash_string` | `size_t stbds_hash_string(char *str, size_t seed)` | YES | `stbds_hash_string` |
| 6 | `stbds_hmdel_key` | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | YES | `stbds_hmdel_key` |
| 7 | `stbds_hmfree_func` | `void stbds_hmfree_func(void *a, size_t elemsize)` | YES | `stbds_hmfree_func` |
| 8 | `stbds_hmget_key` | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | YES | `stbds_hmget_key` |
| 9 | `stbds_hmget_key_ts` | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | YES | `stbds_hmget_key_ts` |
| 10 | `stbds_hmput_default` | `void *stbds_hmput_default(void *a, size_t elemsize)` | YES | `stbds_hmput_default` |
| 11 | `stbds_hmput_key` | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | YES | `stbds_hmput_key` |
| 12 | `stbds_rand_seed` | `void stbds_rand_seed(size_t seed)` | YES | `stbds_rand_seed` |
| 13 | `stbds_shmode_func` | `void *stbds_shmode_func(size_t elemsize, int mode)` | YES | `stbds_shmode_func` |
| 14 | `stbds_stralloc` | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | YES | `stbds_stralloc` |
| 15 | `stbds_strreset` | `void stbds_strreset(stbds_string_arena *a)` | YES | `stbds_strreset` |
| 16 | `strkey` | `char *strkey(int n)` | YES | `strkey` |

**Missing from Rust `.so`: 0. Extra in Rust `.so`: 0.**

## Deliberately NOT exported (matches C)

| C entity | why not a dynamic symbol |
|----------|--------------------------|
| `stbds_unit_tests` | declared `extern` in lib.c, never defined; absent from the C `.so` |
| `stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`, `stbds_strdup` | `static` in C → internal in Rust |
| `buffer` (`static char buffer[256]`) | `static` in C → `BUFFER` private static in Rust |
| `stbds_array_header`, `stbds_hash_bucket`, `stbds_hash_index`, `stbds_string_block`, `stbds_string_arena`, `stbds_struct`, `stbds_struct2` | types, not symbols |
| all `arr*`/`hm*`/`sh*` names in lib.c | preprocessor macros — no code is emitted for them; the Rust tests re-implement the macro expansions on the caller side |

## Undefined (imported) symbols

Both `.so`s import only libc. The Rust `.so` additionally imports libc/`libgcc`
symbols pulled in by the Rust standard library (`_Unwind_*`, `pthread_key_*`,
`dl_iterate_phdr`, `mmap64`, …). No non-libc *user* symbol is undefined in
either library, so the parity requirement ("0 missing/undefined non-libc
symbols") holds.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is equivalent).
Verified by `grep -n '^\[features\]' translation/Cargo.toml` → no match.
