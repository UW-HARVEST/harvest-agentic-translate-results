# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```
nm -D --defined-only c_src/build/libharvest-work-xA5qFF.so   | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libstr_dups_lib.so | awk '{print $3}' | sort
comm -23 c.txt rust.txt      # missing from Rust
```

`c_src/src/lib.c` is a single-file amalgamation of `stb_ds.h` plus the `str_dups`
driver from stb_ds's unit-test block. The public header (`include/lib.h`)
declares only `str_dups`, but every non-`static` definition in `lib.c` has
external linkage and therefore appears in `nm -D`. All 16 are listed below.

| # | symbol | C signature | Rust export | status |
|---|--------|-------------|-------------|--------|
| 1 | `stbds_arrgrowf` | `void *(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | yes | OK |
| 2 | `stbds_arrfreef` | `void (void *a)` | yes | OK |
| 3 | `stbds_rand_seed` | `void (size_t seed)` | yes | OK |
| 4 | `stbds_hash_string` | `size_t (char *str, size_t seed)` | yes | OK |
| 5 | `stbds_hash_bytes` | `size_t (void *p, size_t len, size_t seed)` | yes | OK |
| 6 | `stbds_hmfree_func` | `void (void *a, size_t elemsize)` | yes | OK |
| 7 | `stbds_hmget_key_ts` | `void *(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | yes | OK |
| 8 | `stbds_hmget_key` | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | OK |
| 9 | `stbds_hmput_default` | `void *(void *a, size_t elemsize)` | yes | OK |
| 10 | `stbds_hmput_key` | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | OK |
| 11 | `stbds_shmode_func` | `void *(size_t elemsize, int mode)` | yes | OK |
| 12 | `stbds_hmdel_key` | `void *(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | yes | OK |
| 13 | `stbds_stralloc` | `char *(stbds_string_arena *a, char *str)` | yes | OK |
| 14 | `stbds_strreset` | `void (stbds_string_arena *a)` | yes | OK |
| 15 | `strkey` | `char *(int n)` | yes | OK |
| 16 | `str_dups` | `void (int num)` | yes | OK |

## `static` (internal) C functions — deliberately NOT exported by either .so

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`, plus the `static size_t stbds_hash_seed` /
`static char buffer[256]` objects. The Rust translation keeps all of these
private (`fn` / `static mut`), matching the C `.so`'s `nm -D`.

The C source also `extern`-declares `stbds_unit_tests` but never defines it; it
is not in `nm -D --defined-only` of either library, so nothing to translate.

## Result

```
$ comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt
(empty)
$ comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt
(empty)
```

**0 missing symbols; 0 extra symbols.** Undefined symbols in the Rust `.so` are
all libc / libgcc-unwind / Rust-runtime imports (`realloc`, `free`, `memset`,
`memcpy`, `memmove`, `bcmp` (LLVM's `memcmp`), `strcmp`, `strlen`, `sprintf`,
`printf`, `__assert_fail`, `abort`, `_Unwind_*`, `__tls_get_addr`, …) — no
undefined `stbds_*` references.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one (`cargo test`,
`cargo test --no-default-features`). Both were exercised; see
`FEATURE_MATRIX.md`.
