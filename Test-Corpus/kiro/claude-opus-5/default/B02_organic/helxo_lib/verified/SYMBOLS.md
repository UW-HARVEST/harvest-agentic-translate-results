# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-9TiVI8.so
nm -D --defined-only translation/target/release/libhelxo_lib.so
```

## Public (dynamic, defined) symbols of the C `.so`

| # | symbol | C signature (from `c_src/src/lib.c`) | exported by Rust `.so`? |
|---|--------|--------------------------------------|-------------------------|
| 1 | `stbds_arrgrowf` | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | YES |
| 2 | `stbds_arrfreef` | `void stbds_arrfreef(void *a)` | YES |
| 3 | `stbds_rand_seed` | `void stbds_rand_seed(size_t seed)` | YES |
| 4 | `stbds_hash_string` | `size_t stbds_hash_string(char *str, size_t seed)` | YES |
| 5 | `stbds_hash_bytes` | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | YES |
| 6 | `stbds_hmfree_func` | `void stbds_hmfree_func(void *a, size_t elemsize)` | YES |
| 7 | `stbds_hmget_key_ts` | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | YES |
| 8 | `stbds_hmget_key` | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | YES |
| 9 | `stbds_hmput_default` | `void *stbds_hmput_default(void *a, size_t elemsize)` | YES |
| 10 | `stbds_hmput_key` | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | YES |
| 11 | `stbds_shmode_func` | `void *stbds_shmode_func(size_t elemsize, int mode)` | YES |
| 12 | `stbds_hmdel_key` | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | YES |
| 13 | `stbds_stralloc` | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | YES |
| 14 | `stbds_strreset` | `void stbds_strreset(stbds_string_arena *a)` | YES |
| 15 | `strkey` | `char *strkey(int n)` | YES |
| 16 | `helxo` | `void helxo(char letter)` (the only symbol in `include/lib.h`) | YES |

## Symbols declared `extern` in the C but NOT defined / NOT exported

These are declared in `lib.c` but never defined, so they do not appear in
`nm -D --defined-only` for the C `.so`, and must NOT be exported by Rust:

* `stbds_unit_tests` (declared at line 83, no definition)

## `static` (internal, non-exported) C functions — correctly not exported

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`. All present as private Rust `fn`s.

## Diff result

```
comm -23 <(nm -D --defined-only C.so  | awk '{print $3}' | sort) \
         <(nm -D --defined-only RS.so | awk '{print $3}' | sort)
```

→ **EMPTY**. 0 symbols missing from the Rust `.so`.

Undefined-symbol check on the Rust `.so`: all `U` entries are libc
(`malloc`, `realloc`, `free`, `memset`, `memcpy`, `memmove`, `strcmp`,
`strlen`, `printf`, `sprintf`, plus Rust-runtime/`std`/unwind imports:
`_Unwind_*`, `pthread_*`, `dl_iterate_phdr`, `mmap64`, …).
**0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. Verified with:

```
grep -n '^\[features\]' translation/Cargo.toml   # no match
```

Phase D's "repeat B–C for every feature combination" therefore reduces to the
single default configuration; additionally re-run with
`--no-default-features` (equivalent, no features exist).

## Verification (re-run 2026-09-05, after the `STBDS_ASSERT` fix)

```
$ nm -D --defined-only c_src/build/libharvest-work-9TiVI8.so | wc -l
16
$ nm -D --defined-only translation/target/release/libhelxo_lib.so | wc -l
16
$ comm -3 <(nm -D --defined-only C.so  | awk '{print $3}' | sort) \
          <(nm -D --defined-only RS.so | awk '{print $3}' | sort)
        # (empty in BOTH directions)
```

Every one of the 23 functions in `c_src/src/lib.c` (16 external + 7 `static`)
has a counterpart in `translation/src/lib.rs`; `grep` for
`unimplemented!|todo!|panic!("not` finds nothing, so no symbol is a stub.

Confirmed under all four build configurations by `translation/run_all.sh`
(release/debug × default/`--no-default-features`).

## External-caller sanity check

Beyond the `libloading` suite, the same C driver was linked against each `.so`
in turn and its stdout compared:

```
$ cat drv.c
#include <stdio.h>
void helxo(char);
int main(void){ helxo('Z'); return 0; }

$ gcc -o drv_c drv.c -L c_src/build            -l:libharvest-work-9TiVI8.so ...
$ gcc -o drv_r drv.c -L translation/target/release -l:libhelxo_lib.so       ...
$ diff <(./drv_c) <(./drv_r) && echo IDENTICAL
IDENTICAL          # "bob h\nsally e\nfred l\njen Z\ndoug o\n"
```

This also confirms the Rust reproduces the C's `printf("%s %c\n", hash[z],
hash[z].value)` — where the whole 16-byte element struct is passed as the `%s`
argument, so under the SysV ABI `%s` consumes the `key` pointer from the first
eightbyte and `%c` consumes the low byte of the second (the `value`), leaving the
explicit third argument unused.
