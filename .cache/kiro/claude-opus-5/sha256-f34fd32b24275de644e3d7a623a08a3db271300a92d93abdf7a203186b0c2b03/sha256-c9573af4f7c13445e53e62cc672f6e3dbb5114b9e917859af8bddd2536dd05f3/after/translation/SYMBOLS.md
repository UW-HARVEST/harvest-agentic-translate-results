# SYMBOLS.md — public ABI surface

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-TltUpD.so | awk '{print $2, $3}' | sort
nm -D --defined-only translation/target/release/libstr_put_lib.so | awk '{print $2, $3}' | sort
```

`c_src/include/lib.h` declares only `void str_put(int num);`, so the rest of the
ABI comes from the non-`static` definitions inside `c_src/src/lib.c`.

## Defined (exported) symbols

| # | symbol | C type / signature | C source | Rust impl | exported by Rust `.so` |
|---|--------|--------------------|----------|-----------|------------------------|
| 1 | `stbds_arrgrowf`      | `void *(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | lib.c:271 | `src/arr.rs` | yes |
| 2 | `stbds_arrfreef`      | `void (void *a)` | lib.c:305 | `src/arr.rs` | yes |
| 3 | `stbds_rand_seed`     | `void (size_t seed)` | lib.c:359 | `src/hash.rs` | yes |
| 4 | `stbds_hash_string`   | `size_t (char *str, size_t seed)` | lib.c:481 | `src/hash.rs` | yes |
| 5 | `stbds_hash_bytes`    | `size_t (void *p, size_t len, size_t seed)` | lib.c:553 | `src/hash.rs` | yes |
| 6 | `stbds_hmfree_func`   | `void (void *a, size_t elemsize)` | lib.c:572 | `src/hash.rs` | yes |
| 7 | `stbds_hmget_key_ts`  | `void *(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | lib.c:634 | `src/hash.rs` | yes |
| 8 | `stbds_hmget_key`     | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | lib.c:663 | `src/hash.rs` | yes |
| 9 | `stbds_hmput_default` | `void *(void *a, size_t elemsize)` | lib.c:670 | `src/hash.rs` | yes |
| 10 | `stbds_hmput_key`    | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | lib.c:682 | `src/hash.rs` | yes |
| 11 | `stbds_shmode_func`  | `void *(size_t elemsize, int mode)` | lib.c:798 | `src/hash.rs` | yes |
| 12 | `stbds_hmdel_key`    | `void *(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | lib.c:808 | `src/hash.rs` | yes |
| 13 | `stbds_stralloc`     | `char *(stbds_string_arena *a, char *str)` | lib.c:883 | `src/strings.rs` | yes |
| 14 | `stbds_strreset`     | `void (stbds_string_arena *a)` | lib.c:920 | `src/strings.rs` | yes |
| 15 | `strkey`             | `char *(int n)` | lib.c:941 | `src/testapi.rs` | yes |
| 16 | `str_put`            | `void (int num)` | lib.c:947 | `src/testapi.rs` | yes |

`diff` of the two sorted symbol lists is **empty** — 16 defined symbols on both
sides, identical names and identical `T` binding.

## Intentionally NOT exported (matches C)

| C entity | why not exported |
|----------|------------------|
| `static char buffer[256]` | file-scope `static` → local symbol (`b`/`d` in `nm`, absent from `nm -D`) |
| `stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`, `stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`, `stbds_strdup` | `static` in the C source |
| `stbds_unit_tests` | `extern`-declared in lib.c but never defined; not in the `.so` |

## Undefined (imported) symbols

The Rust `.so` imports only libc / libgcc names
(`realloc`, `free`, `memmove`, `memcpy`, `memset`, `bcmp`/`memcmp`, `strlen`,
`strcmp`, `printf`, `sprintf`, `__assert_fail`, `abort`, plus the
`_Unwind_*` / `__cxa_*` / `pthread_*` runtime support that `libstd` pulls in).
**0 missing / undefined non-libc symbols.**

`tests/symbols.rs` enforces both halves of this automatically:

* `defined_symbol_diff_is_empty` — `nm -D --defined-only` on both objects, then
  `C \ Rust` must be empty, the C count must be exactly 16, and each of the 16
  names must be present on both sides.
* `rust_so_imports_only_runtime_symbols` — every `nm -D --undefined-only` entry
  must be an allow-listed libc / runtime name, and in particular **no `stbds_*`
  symbol may be undefined**, which is what would happen if a C module had been
  left untranslated and merely re-declared.

`tests/feature_matrix.sh` repeats the `nm -D` diff for every feature
combination.
