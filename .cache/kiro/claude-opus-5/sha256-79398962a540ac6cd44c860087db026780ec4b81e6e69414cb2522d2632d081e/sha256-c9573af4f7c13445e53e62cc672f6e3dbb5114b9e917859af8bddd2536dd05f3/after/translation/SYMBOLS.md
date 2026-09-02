# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-S9Bccf.so | awk '{print $3}' | sort -u
nm -D --defined-only translation/target/release/libsh_puts_lib.so | awk '{print $3}' | sort -u
```

The C library is built from a single translation unit (`c_src/src/lib.c`, see
`c_src/CMakeLists.txt`); there is no second module, so there is no missing-module
completeness gap. `c_src/include/lib.h` declares only `sh_puts`; everything else
is `extern`-declared inside `lib.c` itself and reaches the dynamic symbol table
because it has external linkage.

## C dynamic symbols (16) vs Rust

| # | C symbol | C signature (from `lib.c`) | in Rust `.so` | Rust item |
|---|----------|----------------------------|---------------|-----------|
| 1 | `sh_puts` | `void sh_puts(int num)` | YES | `sh_puts` |
| 2 | `stbds_arrfreef` | `void stbds_arrfreef(void *a)` | YES | `stbds_arrfreef` |
| 3 | `stbds_arrgrowf` | `void *stbds_arrgrowf(void*, size_t elemsize, size_t addlen, size_t min_cap)` | YES | `stbds_arrgrowf` |
| 4 | `stbds_hash_bytes` | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | YES | `stbds_hash_bytes` |
| 5 | `stbds_hash_string` | `size_t stbds_hash_string(char *str, size_t seed)` | YES | `stbds_hash_string` |
| 6 | `stbds_hmdel_key` | `void *stbds_hmdel_key(void*, size_t, void*, size_t, size_t keyoffset, int mode)` | YES | `stbds_hmdel_key` |
| 7 | `stbds_hmfree_func` | `void stbds_hmfree_func(void *p, size_t elemsize)` | YES | `stbds_hmfree_func` |
| 8 | `stbds_hmget_key` | `void *stbds_hmget_key(void*, size_t, void*, size_t, int mode)` | YES | `stbds_hmget_key` |
| 9 | `stbds_hmget_key_ts` | `void *stbds_hmget_key_ts(void*, size_t, void*, size_t, ptrdiff_t *temp, int mode)` | YES | `stbds_hmget_key_ts` |
| 10 | `stbds_hmput_default` | `void *stbds_hmput_default(void *a, size_t elemsize)` | YES | `stbds_hmput_default` |
| 11 | `stbds_hmput_key` | `void *stbds_hmput_key(void*, size_t, void*, size_t, int mode)` | YES | `stbds_hmput_key` |
| 12 | `stbds_rand_seed` | `void stbds_rand_seed(size_t seed)` | YES | `stbds_rand_seed` |
| 13 | `stbds_shmode_func` | `void *stbds_shmode_func(size_t elemsize, int mode)` | YES | `stbds_shmode_func` |
| 14 | `stbds_stralloc` | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | YES | `stbds_stralloc` |
| 15 | `stbds_strreset` | `void stbds_strreset(stbds_string_arena *a)` | YES | `stbds_strreset` |
| 16 | `strkey` | `char *strkey(int n)` | YES | `strkey` |

**Symbol diff (`comm -23 c_syms r_syms`): EMPTY — 0 missing.**

No stubs / `unimplemented!()` were added: every one of the 16 is a real
translation of the corresponding C body.

## Declared-but-never-defined in C (correctly absent from both)

`lib.c` contains `extern` declarations that the translation unit never defines,
so they are **undefined** in the C `.so` and must not be exported by Rust:

- `stbds_unit_tests` (declared `extern void stbds_unit_tests(void);`, no body,
  never called → does not even appear as an undefined symbol after linking).

## Static (internal-linkage) C functions — intentionally not exported

These are `static` in C and therefore private in Rust as well:
`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`. Also the `static char buffer[256]` and
`static size_t stbds_hash_seed`.

## Undefined-symbol check on the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` yields only libc / libgcc-unwind
imports (`realloc`, `free`, `strlen`, `strcmp`, `bcmp` (= `memcmp`), `memcpy`,
`memmove`, `memset`, `printf`, plus the Rust std runtime's `malloc`, `calloc`,
`posix_memalign`, `abort`, `write`, `_Unwind_*`, `pthread_*`, `dl_iterate_phdr`,
`__cxa_*`, `stat64`/`open64`/`read`/`mmap64`… and the standard
`_ITM_*` / `__gmon_start__` weak stubs).

**0 missing / undefined non-libc symbols.**

The C `.so` additionally imports `__assert_fail` and `sprintf`:

- `__assert_fail` — `STBDS_ASSERT` is `assert` and CMake's default (empty)
  `CMAKE_BUILD_TYPE` does **not** define `NDEBUG`, so the C asserts are LIVE.
  One of them (`ERRORS.md` E18) is reachable through the public API, so the Rust
  translation reinstates them as `stbds_assert!`, which writes a diagnostic to
  fd 2 and calls libc `abort()`. That is why the Rust `.so` imports `abort` and
  `write` — the SIGABRT/exit-134 behaviour is verified to match in
  `tests/errors.rs::e18_mode2_nonlast_delete_aborts_in_both`. See `ERRORS.md`
  rows E14–E21 for the reachability analysis of each assert.
- `sprintf` — used by `strkey`; the Rust translation formats the integer by
  hand into the same 256-byte static buffer (verified byte-identical in
  `tests/leaf.rs::c10_strkey` and `tests/errors.rs::e45_strkey_extremes`).

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table** and no `default`
features, so there is exactly one build configuration. `verify.sh` extracts the
feature list mechanically and, finding none, checks the three equivalent
invocations (`<default>`, `--no-default-features`, `--all-features`) in both the
`release` and `debug` profiles — 6 configurations, each with its own symbol diff
and a full test run:

```
cd translation && ./verify.sh
# -> ALL CONFIGURATIONS PASS
```

Verified output for every one of the 6: `symbol diff: empty (all 16 C exports
present)`, `undefined: libc/libgcc only`, and 74 passing tests.
