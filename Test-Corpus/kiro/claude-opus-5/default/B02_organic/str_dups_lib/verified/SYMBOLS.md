# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-0huCPf.so
nm -D --defined-only translation/target/release/libstr_dups_lib.so
```

The C library is a single-TU amalgamation of `stb_ds.h` plus the `str_dups`
driver from stb_ds's unit-test block (`c_src/src/lib.c`, 969 lines). Everything
else in the file is a preprocessor macro or a `static` function, so it produces
no dynamic symbol.

## Public symbol table

| # | symbol | C type | Rust wrapper | present in Rust `.so` |
|---|--------|--------|--------------|-----------------------|
| 1 | `stbds_arrgrowf` | `void *(void*, size_t, size_t, size_t)` | `#[no_mangle] extern "C"` | yes |
| 2 | `stbds_arrfreef` | `void (void*)` | `#[no_mangle] extern "C"` | yes |
| 3 | `stbds_rand_seed` | `void (size_t)` | `#[no_mangle] extern "C"` | yes |
| 4 | `stbds_hash_string` | `size_t (char*, size_t)` | `#[no_mangle] extern "C"` | yes |
| 5 | `stbds_hash_bytes` | `size_t (void*, size_t, size_t)` | `#[no_mangle] extern "C"` | yes |
| 6 | `stbds_hmfree_func` | `void (void*, size_t)` | `#[no_mangle] extern "C"` | yes |
| 7 | `stbds_hmget_key` | `void *(void*, size_t, void*, size_t, int)` | `#[no_mangle] extern "C"` | yes |
| 8 | `stbds_hmget_key_ts` | `void *(void*, size_t, void*, size_t, ptrdiff_t*, int)` | `#[no_mangle] extern "C"` | yes |
| 9 | `stbds_hmput_default` | `void *(void*, size_t)` | `#[no_mangle] extern "C"` | yes |
| 10 | `stbds_hmput_key` | `void *(void*, size_t, void*, size_t, int)` | `#[no_mangle] extern "C"` | yes |
| 11 | `stbds_hmdel_key` | `void *(void*, size_t, void*, size_t, size_t, int)` | `#[no_mangle] extern "C"` | yes |
| 12 | `stbds_shmode_func` | `void *(size_t, int)` | `#[no_mangle] extern "C"` | yes |
| 13 | `stbds_stralloc` | `char *(stbds_string_arena*, char*)` | `#[no_mangle] extern "C"` | yes |
| 14 | `stbds_strreset` | `void (stbds_string_arena*)` | `#[no_mangle] extern "C"` | yes |
| 15 | `strkey` | `char *(int)` | `#[no_mangle] extern "C"` | yes |
| 16 | `str_dups` | `void (int)` | `#[no_mangle] extern "C"` | yes |

## Symbols intentionally absent from BOTH libraries

Declared `extern` in `c_src/src/lib.c` but never defined there, so the C `.so`
lists them as **undefined**, not defined. The Rust `.so` must NOT define them
either (and does not):

* `stbds_unit_tests` — declared at lib.c:84, never defined.

`static`-only C functions with no dynamic symbol (translated as private Rust
`fn`s, correctly not exported): `stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`.

## Undefined (imported) symbols

| library | non-libc undefined symbols |
|---------|----------------------------|
| C `.so` | none |
| Rust `.so` | none |

Both import only libc / libgcc / ld symbols. The C `.so` imports `realloc`,
`free`, `malloc`, `memset`, `memcpy`, `memmove`, `memcmp`, `strcmp`, `strlen`,
`sprintf`, `printf`, `__assert_fail`, `__cxa_finalize`, plus the weak
`_ITM_*`/`__gmon_start__` link-time stubs. The Rust `.so` imports the same libc
functions plus the ones the Rust standard library needs (`mmap64`, `munmap`,
`pthread_key_*`, `write`, …), all versioned `@GLIBC…`, and libgcc's
`_Unwind_*@GCC_*`. Nothing outside those categories.

## Status

**Symbol diff is EMPTY in both directions**, for both the `dev` and the
`release` Rust profile. Verified with:

```
diff <(nm -D --defined-only <C.so>  | awk '$2=="T"{print $3}' | sort) \
     <(nm -D --defined-only <RS.so> | awk '$2=="T"{print $3}' | sort)
```

Automated in `phase_d.sh` (section D.2), which also lists any undefined symbol
in the Rust `.so` that is not glibc (`…@GLIBC…`), libgcc unwind
(`_Unwind_*@GCC_*`), or a standard weak stub (`_ITM_*`, `__gmon_start__`) —
that list is empty.

No module of the C source was left untranslated: `c_src/CMakeLists.txt` lists
only `src/lib.c`, and all 16 of its external definitions plus all 7 `static`
helpers are present in `translation/src/lib.rs`. No export is a stub.

- [x] `nm -D` shows 0 missing symbols in Rust (debug and release)
- [x] `nm -D` shows 0 extra symbols in Rust (debug and release)
- [x] 0 undefined non-libc symbols in Rust
- [x] No stubbed / `unimplemented!()` exports
