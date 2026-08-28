# SYMBOLS.md — Public symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-uAHqBm.so
nm -D --defined-only translation/target/release/libintput_lib.so
```

The C library is a single translation unit (`c_src/src/lib.c`) containing the
`stb_ds.h` implementation plus two test helpers (`strkey`, `intput`).
`c_src/include/lib.h` declares only `void intput(int num);` — every other
exported symbol is a non-`static` definition in `lib.c`.

## Exported (dynamic, `T`) symbols

| # | symbol | C signature (from lib.c) | in C `.so` | in Rust `.so` | status |
|---|--------|--------------------------|-----------|---------------|--------|
| 1 | `stbds_arrgrowf`      | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | yes | yes | OK |
| 2 | `stbds_arrfreef`      | `void stbds_arrfreef(void *a)` | yes | yes | OK |
| 3 | `stbds_rand_seed`     | `void stbds_rand_seed(size_t seed)` | yes | yes | OK |
| 4 | `stbds_hash_string`   | `size_t stbds_hash_string(char *str, size_t seed)` | yes | yes | OK |
| 5 | `stbds_hash_bytes`    | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | yes | yes | OK |
| 6 | `stbds_hmfree_func`   | `void stbds_hmfree_func(void *a, size_t elemsize)` | yes | yes | OK |
| 7 | `stbds_hmget_key_ts`  | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | yes | yes | OK |
| 8 | `stbds_hmget_key`     | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes | OK |
| 9 | `stbds_hmput_default` | `void *stbds_hmput_default(void *a, size_t elemsize)` | yes | yes | OK |
|10 | `stbds_hmput_key`     | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes | OK |
|11 | `stbds_shmode_func`   | `void *stbds_shmode_func(size_t elemsize, int mode)` | yes | yes | OK |
|12 | `stbds_hmdel_key`     | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | yes | yes | OK |
|13 | `stbds_stralloc`      | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | yes | yes | OK |
|14 | `stbds_strreset`      | `void stbds_strreset(stbds_string_arena *a)` | yes | yes | OK |
|15 | `strkey`              | `char *strkey(int n)` | yes | yes | OK |
|16 | `intput`              | `void intput(int num)` | yes | yes | OK |

**Symbol diff: EMPTY.** 16 exported symbols in C, 16 in Rust, exact name match.

```
$ diff <(nm -D --defined-only c_src/build/*.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libintput_lib.so | awk '{print $3}' | sort)
(no output)
```

## Not exported by either (correctly)

These are `static` in `lib.c` and therefore local (`t`/`d` in `nm`, absent from
`nm -D`).  The Rust translation keeps them private too.

| C symbol | why not exported |
|----------|------------------|
| `stbds_hash_seed`       | `static size_t` |
| `stbds_probe_position`  | `static` |
| `stbds_log2`            | `static` |
| `stbds_make_hash_index` | `static` |
| `stbds_siphash_bytes`   | `static` |
| `stbds_is_key_equal`    | `static` |
| `stbds_hm_find_slot`    | `static` |
| `stbds_strdup`          | `static` |
| `buffer`                | `static char buffer[256]` |

`stbds_unit_tests` is *declared* `extern` at lib.c:83 but never defined, so it
appears in neither `.so`.  Rust correctly does not define it.

## Undefined symbols in the Rust `.so`

`nm -D -u translation/target/release/libintput_lib.so` lists only libc /
libgcc-unwind imports (`realloc`, `free`, `memcpy`, `memmove`, `memset`,
`__assert_fail`, `abort`, `_Unwind_*`, `dl_iterate_phdr`, …).  **0 missing or
undefined non-libc symbols.**

## Assertion string parity

`assert()` diagnostics are part of the observable behaviour (they are printed to
stderr before `abort()`).  All six distinct assertion strings plus the four
`__PRETTY_FUNCTION__` names and the `__FILE__` path exist in both `.so`s:

| assertion text | C `.so` | Rust `.so` |
|---|---|---|
| `t->used_count_threshold + t->tombstone_count_threshold < t->slot_count` | yes | yes |
| `(size_t) i+1 <= stbds_arrcap(a)` | yes | yes |
| `slot < (ptrdiff_t) table->slot_count` | yes | yes |
| `slot >= 0` | yes | yes |
| `b->index[i] == final_index` | yes | yes |
| `len <= a->remaining` | yes | yes |
| `hmget(intmap, 9) == num` | yes | yes |
| `hmget(intmap, 11) == 3` | yes | yes |
| `hmget(intmap, num) == 7` | yes | yes |
| `/…/c_src/src/lib.c` (`__FILE__`) | yes | yes (via `build.rs`) |
