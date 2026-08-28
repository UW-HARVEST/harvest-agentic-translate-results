# SYMBOLS.md — public surface parity (Phase A / Phase D)

The C library is built from the single translation unit `c_src/src/lib.c`
(`cmake` target name is derived from the parent directory name, so the artifact
is `c_src/build/lib<workdir>.so`).  The Rust crate builds `cdylib`
`translation/target/{debug,release}/libhm_geti_lib.so`.

Commands used:

```sh
nm -D --defined-only c_src/build/lib*.so            | awk '{print $NF}' | sort
nm -D --defined-only translation/target/release/libhm_geti_lib.so | awk '{print $NF}' | sort
```

## Defined (exported) symbols

| # | symbol | C signature (from `lib.c`) | in C `.so` | in Rust `.so` |
|---|--------|----------------------------|-----------|---------------|
| 1 | `stbds_arrgrowf`    | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | yes | yes |
| 2 | `stbds_arrfreef`    | `void stbds_arrfreef(void *a)` | yes | yes |
| 3 | `stbds_rand_seed`   | `void stbds_rand_seed(size_t seed)` | yes | yes |
| 4 | `stbds_hash_string` | `size_t stbds_hash_string(char *str, size_t seed)` | yes | yes |
| 5 | `stbds_hash_bytes`  | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | yes | yes |
| 6 | `stbds_hmfree_func` | `void stbds_hmfree_func(void *a, size_t elemsize)` | yes | yes |
| 7 | `stbds_hmget_key_ts`| `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | yes | yes |
| 8 | `stbds_hmget_key`   | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes |
| 9 | `stbds_hmput_default`| `void *stbds_hmput_default(void *a, size_t elemsize)` | yes | yes |
| 10 | `stbds_hmput_key`  | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes |
| 11 | `stbds_shmode_func`| `void *stbds_shmode_func(size_t elemsize, int mode)` | yes | yes |
| 12 | `stbds_hmdel_key`  | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | yes | yes |
| 13 | `stbds_stralloc`   | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | yes | yes |
| 14 | `stbds_strreset`   | `void stbds_strreset(stbds_string_arena *a)` | yes | yes |
| 15 | `strkey`           | `char *strkey(int n)` | yes | yes |
| 16 | `hm_geti`          | `void hm_geti(int num)` (the only symbol in `include/lib.h`) | yes | yes |

`diff` of the two sorted name lists is **empty**: 16 symbols on each side, no
missing and no extra names.

## Not exported by either side (verified)

`lib.c` declares but never defines `stbds_unit_tests`; it is not referenced, so
neither `.so` contains it.  These are `static` in C and therefore correctly
absent from both `.so`s (they are translated as private Rust `fn`s):

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`, and the file-static data `stbds_hash_seed`, `buffer`.

## Undefined (imported) symbols

The C `.so` imports only libc: `__assert_fail`, `free`, `malloc`, `memcmp`,
`memcpy`, `memmove`, `memset`, `realloc`, `sprintf`, `strcmp`, `strlen`
(plus the usual `_ITM_*`, `__cxa_finalize`, `__gmon_start__` stubs).

The Rust `.so` imports the same libc set (`free`, `malloc`, `realloc`,
`memcmp`/`bcmp`, `memcpy`, `memmove`, `memset`, `strcmp`, `strlen`, `abort`)
plus the standard Rust runtime imports (`_Unwind_*`, `pthread_key_*`,
`__tls_get_addr`, `dl_iterate_phdr`, io syscalls used by the panic/backtrace
machinery).  **0 undefined non-libc/non-runtime symbols** — nothing is missing
at link time (`ldd` resolves the library fully).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the crate has
exactly one configuration (the default).  `cargo check`/`cargo test` with
`--no-default-features` is therefore equivalent to the default build; both are
run by `scratch/run_all.sh` for completeness, as are the `dev` and `release`
profiles (the profile matters because `[profile.release]` sets
`overflow-checks = false`, so the release build is the one whose wrapping
arithmetic must match C).
