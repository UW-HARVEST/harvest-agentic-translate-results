# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-KkkAp8.so   | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libstr_dups_lib.so | awk '{print $3}' | sort
```

The C library is built from the single translation unit `c_src/src/lib.c`
(see `c_src/CMakeLists.txt`); `c_src/include/lib.h` declares only
`void str_dups(int num);`, but the translation unit gives **external
linkage** to 16 symbols. All of them are part of the verified surface.

## Exported (dynamic, defined) symbols

| # | symbol | C signature (from `c_src/src/lib.c`) | in C `.so` | in Rust `.so` | status |
|---|--------|--------------------------------------|-----------|--------------|--------|
| 1 | `stbds_arrgrowf`      | `void * stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | yes | yes | OK |
| 2 | `stbds_arrfreef`      | `void stbds_arrfreef(void *a)` | yes | yes | OK |
| 3 | `stbds_rand_seed`     | `void stbds_rand_seed(size_t seed)` | yes | yes | OK |
| 4 | `stbds_hash_string`   | `size_t stbds_hash_string(char *str, size_t seed)` | yes | yes | OK |
| 5 | `stbds_hash_bytes`    | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | yes | yes | OK |
| 6 | `stbds_hmfree_func`   | `void stbds_hmfree_func(void *p, size_t elemsize)` | yes | yes | OK |
| 7 | `stbds_hmget_key_ts`  | `void * stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | yes | yes | OK |
| 8 | `stbds_hmget_key`     | `void * stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes | OK |
| 9 | `stbds_hmput_default` | `void * stbds_hmput_default(void *a, size_t elemsize)` | yes | yes | OK |
| 10 | `stbds_hmput_key`    | `void * stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes | OK |
| 11 | `stbds_shmode_func`  | `void * stbds_shmode_func(size_t elemsize, int mode)` | yes | yes | OK |
| 12 | `stbds_hmdel_key`    | `void * stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | yes | yes | OK |
| 13 | `stbds_stralloc`     | `char * stbds_stralloc(stbds_string_arena *a, char *str)` | yes | yes | OK |
| 14 | `stbds_strreset`     | `void stbds_strreset(stbds_string_arena *a)` | yes | yes | OK |
| 15 | `strkey`             | `char * strkey(int n)` | yes | yes | OK |
| 16 | `str_dups`           | `void str_dups(int num)` | yes | yes | OK |

**Symbol diff: EMPTY in both directions.**

```
comm -23 c_names.txt rust_names.txt   ->   (empty)   # in C, missing from Rust
comm -13 c_names.txt rust_names.txt   ->   (empty)   # extra in Rust
16 c_names.txt
16 rust_names.txt
```

## Declared-but-never-defined C externs (correctly absent from both `.so`s)

`c_src/src/lib.c` forward-declares these with `extern` but the file never
defines them, so the C `.so` has no definition either. The Rust side must
likewise *not* invent them (a stub would lie about behaviour):

* `stbds_unit_tests` (line 83) — never defined, never called.

## `static` C objects (internal linkage, correctly not exported by either side)

| C name | kind |
|--------|------|
| `stbds_hash_seed` | `static size_t` global mutable seed |
| `stbds_probe_position` | `static` fn |
| `stbds_log2` | `static` fn |
| `stbds_make_hash_index` | `static` fn |
| `stbds_siphash_bytes` | `static` fn |
| `stbds_is_key_equal` | `static` fn |
| `stbds_hm_find_slot` | `static` fn |
| `stbds_strdup` | `static` fn |
| `buffer` | `static char[256]` used by `strkey` |

## Undefined (imported) symbols

C `.so` imports only libc: `__assert_fail memcmp memcpy memmove memset
printf realloc malloc free sprintf strcmp strlen` (+ weak
`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Rust `.so` imports only libc / libgcc unwinder:
`realloc free malloc calloc posix_memalign memcpy memmove memset bcmp
strcmp strlen printf sprintf abort __errno_location ...` plus the
`_Unwind_*` personality symbols and `dl_iterate_phdr`/`syscall`/etc. that the
Rust `std` runtime always pulls in.

**0 missing / undefined non-libc symbols in the Rust `.so`.**  ✅

## Verified checklist

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so`
      with the exact same name.
- [x] No extra public symbols in the Rust `.so`.
- [x] No stubs / `unimplemented!()` — every export is a real translation of
      the corresponding C function body. Each of the 16 exports is exercised by
      at least one differential test (see the row → test maps in `CONFIGS.md`
      and `ERRORS.md`).
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Re-verified automatically for **every** feature combination × build
      profile by `./run_all_configs.sh`.

## Per-symbol test coverage

| symbol | driven by |
|--------|-----------|
| `stbds_arrgrowf`      | `cfg01`-`cfg07`, `err_01`-`err_05` (and indirectly by every map test) |
| `stbds_arrfreef`      | `cfg01`-`cfg07`, `err_59_arrfreef_null_aborts_identically` |
| `stbds_rand_seed`     | `cfg14`, `t03_seed_advance_identical`, and every map test (used to sync the process-global seed) |
| `stbds_hash_string`   | `cfg11`-`cfg13`, `err_50`, `err_51`, `err_52`, `err_60` |
| `stbds_hash_bytes`    | `cfg08`-`cfg10`, `err_48`, `err_49` |
| `stbds_hmfree_func`   | `cfg48`, `err_06`, `err_07`, plus teardown of every map test |
| `stbds_hmget_key_ts`  | `cfg29`, `cfg30`, `cfg44`, `err_10`, `err_11`, `cfg54`/`cfg55` fuzz |
| `stbds_hmget_key`     | `cfg23`-`cfg29`, `cfg44`, `err_12`, `err_13`, `err_53` |
| `stbds_hmput_default` | `cfg31`, `err_14_15_16`, `cfg54`/`cfg55` fuzz |
| `stbds_hmput_key`     | `cfg23`-`cfg28`, `cfg37`-`cfg43b`, `cfg49`-`cfg51`, `err_17`-`err_24`, `err_53`-`err_55` |
| `stbds_shmode_func`   | `cfg14`, `cfg37`-`cfg40`, `cfg49`, `cfg51`, `err_25`, `err_26` |
| `stbds_hmdel_key`     | `cfg27`, `cfg32`-`cfg36`, `cfg45`-`cfg47`, `err_27`-`err_39`, `err_34` |
| `stbds_stralloc`      | `cfg15`-`cfg22`, `cfg39`, `cfg40`, `err_41`-`err_46` |
| `stbds_strreset`      | `cfg15`-`cfg22`, `err_47` (and via `stbds_hmfree_func`) |
| `strkey`              | `cfg52`, `err_58` |
| `str_dups`            | `cfg53`, `cfg53b`, `err_56_57` (stdout captured with `dup2` and compared byte-for-byte) |
