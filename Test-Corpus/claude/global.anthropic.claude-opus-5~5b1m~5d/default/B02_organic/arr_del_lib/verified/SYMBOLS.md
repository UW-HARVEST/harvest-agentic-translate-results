# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on
`c_src/build/libharvest-work-Tzy9eC.so` (C) vs
`translation/target/release/libarr_del_lib.so` (Rust).

The C library is built from a single TU (`c_src/src/lib.c`), an inlined copy of
`stb_ds.h` plus the two file-local helpers `strkey` / `arr_del`. Only 16 symbols
have external linkage; everything else (`stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`, `buffer`, `stbds_hash_seed`) is `static`.

## Exported (T) symbols

| # | symbol | C | Rust | signature |
|---|--------|---|------|-----------|
| 1 | `arr_del` | T | T | `void arr_del(int num)` |
| 2 | `stbds_arrfreef` | T | T | `void stbds_arrfreef(void *a)` |
| 3 | `stbds_arrgrowf` | T | T | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` |
| 4 | `stbds_hash_bytes` | T | T | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` |
| 5 | `stbds_hash_string` | T | T | `size_t stbds_hash_string(char *str, size_t seed)` |
| 6 | `stbds_hmdel_key` | T | T | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` |
| 7 | `stbds_hmfree_func` | T | T | `void stbds_hmfree_func(void *p, size_t elemsize)` |
| 8 | `stbds_hmget_key` | T | T | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` |
| 9 | `stbds_hmget_key_ts` | T | T | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` |
| 10 | `stbds_hmput_default` | T | T | `void *stbds_hmput_default(void *a, size_t elemsize)` |
| 11 | `stbds_hmput_key` | T | T | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` |
| 12 | `stbds_rand_seed` | T | T | `void stbds_rand_seed(size_t seed)` |
| 13 | `stbds_shmode_func` | T | T | `void *stbds_shmode_func(size_t elemsize, int mode)` |
| 14 | `stbds_stralloc` | T | T | `char *stbds_stralloc(stbds_string_arena *a, char *str)` |
| 15 | `stbds_strreset` | T | T | `void stbds_strreset(stbds_string_arena *a)` |
| 16 | `strkey` | T | T | `char *strkey(int n)` |

## Diff result

```
$ diff <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only rust.so| awk '{print $3}' | sort)
(empty)
```

- Missing from Rust: **none**
- Extra in Rust: **none**
- Undefined non-libc symbols in the Rust `.so`: **none**
  (`nm -D -u` lists only glibc + `_Unwind_*`/`__gmon_start__`/`_ITM_*` weak refs,
  all of which the C `.so` also pulls in from libc/libgcc.)

## Notes / not exported (deliberately)

`arr_del` is the only symbol in `include/lib.h`; the `stbds_*` symbols are
`extern`-declared inside `lib.c` and get external linkage from their
definitions. `strkey` is non-static in the C and therefore also exported.

The C build has no `-fvisibility=hidden`, so this list is the complete surface.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` section** — there is exactly
one build configuration (`--no-default-features` is identical to the default).
Verified with `cargo check --no-default-features`. Phase D's "every feature
combination" therefore collapses to the single default configuration, which is
the one all tests run under.

## Verification status (Phase D)

Re-checked by `./verify.sh`, for **both** the `release` and the `debug` Rust
`.so`, under **every** cargo feature combination (`__default__`, `__none__` —
there are no others):

```
[ok] symbol parity release [__default__]: 16/16 symbols, diff empty
[ok] no undefined non-libc symbols release [__default__]
[ok] symbol parity debug   [__default__]: 16/16 symbols, diff empty
[ok] no undefined non-libc symbols debug   [__default__]
[ok] symbol parity release [__none__]:    16/16 symbols, diff empty
[ok] no undefined non-libc symbols release [__none__]
[ok] symbol parity debug   [__none__]:    16/16 symbols, diff empty
[ok] no undefined non-libc symbols debug   [__none__]
```

Nothing had to be translated to close the gap — the Rust crate already covered
the whole of `c_src/src/lib.c` (there is only one C translation unit, and it has
only these 16 external definitions). No symbol is stubbed or
`unimplemented!()`; every export is a real translation exercised by the
differential tests, which call it **only** through `dlopen`/`dlsym` on the
`.so`, never as a Rust function.

## How the exports are reached from the tests

`tests/common/mod.rs` `dlopen`s both `.so`s and resolves all 16 names into raw
`extern "C"` function pointers, so the `#[unsafe(no_mangle)] pub unsafe extern
"C"` wrappers are on the tested path. A missing or misnamed export fails at
`Lib::open` with `missing symbol <name>`.
