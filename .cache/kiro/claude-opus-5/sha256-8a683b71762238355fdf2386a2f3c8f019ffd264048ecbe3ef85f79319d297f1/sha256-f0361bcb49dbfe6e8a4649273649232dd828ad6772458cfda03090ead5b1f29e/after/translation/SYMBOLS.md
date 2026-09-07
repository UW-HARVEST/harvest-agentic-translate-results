# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Source of truth: `nm -D --defined-only` on both shared objects.

* C  : `c_src/build/libharvest-work-6AiN81.so`
* Rust: `translation/target/release/libarr_ins_lib.so`

## Public symbols exported by the C `.so`

| # | symbol | C definition (`c_src/src/lib.c`) | exported by Rust `.so` |
|---|--------|----------------------------------|------------------------|
| 1 | `arr_ins`             | L942 (the only symbol in `include/lib.h`) | yes |
| 2 | `stbds_arrfreef`      | L297 | yes |
| 3 | `stbds_arrgrowf`      | L262 | yes |
| 4 | `stbds_hash_bytes`    | L555 | yes |
| 5 | `stbds_hash_string`   | L476 | yes |
| 6 | `stbds_hmdel_key`     | L813 | yes |
| 7 | `stbds_hmfree_func`   | L577 | yes |
| 8 | `stbds_hmget_key`     | L668 | yes |
| 9 | `stbds_hmget_key_ts`  | L637 | yes |
|10 | `stbds_hmput_default` | L675 | yes |
|11 | `stbds_hmput_key`     | L688 | yes |
|12 | `stbds_rand_seed`     | L387 | yes |
|13 | `stbds_shmode_func`   | L802 | yes |
|14 | `stbds_stralloc`      | L890 | yes |
|15 | `stbds_strreset`      | L925 | yes |
|16 | `strkey`              | L936 | yes |

## `static` / internal C symbols (correctly NOT exported by either .so)

`stbds_hash_seed`, `buffer`, `stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`.

Declared `extern` in the C prologue but never defined (so absent from the C
`.so` and legitimately absent from the Rust `.so`): `stbds_unit_tests`.

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libharvest-work-6AiN81.so | awk '{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/libarr_ins_lib.so | awk '{print $3}' | sort)
<empty>
```

**0 symbols missing from the Rust `.so`.** The reverse diff is empty too — the
two `.so`s export exactly the same 16 names, no more and no fewer:

```
$ comm -13 <c-syms> <rust-syms>
<empty>
$ nm -D --defined-only c_src/build/lib*.so | wc -l              # 16
$ nm -D --defined-only translation/target/release/*.so | wc -l  # 16
```

## Undefined symbols in the Rust `.so`

All undefined symbols resolve to libc / libgcc_s / ld.so:
`realloc, free, malloc, calloc, memcpy, memmove, memset, bcmp, strlen,
posix_memalign, abort, __errno_location, __cxa_finalize,
__cxa_thread_atexit_impl, __tls_get_addr, pthread_key_*, getenv, getcwd,
readlink, realpath, open64, close, read, write, writev, lseek64, fstat64,
stat64, statx, mmap64, munmap, dl_iterate_phdr, syscall, gettid,
_Unwind_* (GCC_*), _ITM_*TMCloneTable, __gmon_start__`.

**0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. Verified mechanically:

```
$ grep -n '^\[features\]' translation/Cargo.toml   # no match
```

`--no-default-features` is therefore equivalent to the default build and is
also exercised by `scripts/check_features.sh`.

## How the exports are tested

The differential tests never call the Rust crate directly. Both `.so`s are
loaded with `libloading` using `RTLD_NOW | RTLD_LOCAL` (`RTLD_LOCAL` is
essential — with `RTLD_GLOBAL` the two libraries would interpose each other's
identically named symbols), and every one of the 16 symbols is resolved by
`dlsym` and called through an `extern "C"` function pointer. That means the
`#[no_mangle]` wrappers themselves are under test, not just the internal Rust
implementations. See `tests/common/mod.rs`.

## Re-verification

`scripts/check_features.sh` rebuilds both libraries, re-runs the `nm -D` diff
(forward and undefined-symbol filter) and the full suite for every feature
combination. Latest run:

```
=== combination: --all-features ===        OK: 16 C symbols, 0 missing from Rust
=== combination: (default features) ===    OK: 16 C symbols, 0 missing from Rust
=== combination: --no-default-features === OK: 16 C symbols, 0 missing from Rust
ALL FEATURE COMBINATIONS PASSED
```
