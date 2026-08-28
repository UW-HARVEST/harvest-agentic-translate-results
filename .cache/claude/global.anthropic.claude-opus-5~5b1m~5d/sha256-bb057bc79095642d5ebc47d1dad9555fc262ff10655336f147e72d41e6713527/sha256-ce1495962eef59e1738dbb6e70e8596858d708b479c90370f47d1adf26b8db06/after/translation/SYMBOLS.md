# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-DNGbvF.so
nm -D --defined-only translation/target/release/libstr_put_lib.so
```

The C library is built from a single translation unit (`c_src/src/lib.c`), which
is an inlined copy of `stb_ds.h` (implementation part) plus the `strkey` /
`str_put` driver code. `c_src/include/lib.h` declares only `void str_put(int)`,
but the `.so` exports every non-`static` definition in `lib.c`.

## Exported (dynamic, defined) symbols

| # | C symbol | C decl | present in Rust `.so` | Rust item |
|---|----------|--------|-----------------------|-----------|
| 1 | `stbds_arrgrowf`     | `void * (void*, size_t, size_t, size_t)`                        | YES | `stbds_arrgrowf` |
| 2 | `stbds_arrfreef`     | `void (void*)`                                                  | YES | `stbds_arrfreef` |
| 3 | `stbds_rand_seed`    | `void (size_t)`                                                 | YES | `stbds_rand_seed` |
| 4 | `stbds_hash_string`  | `size_t (char*, size_t)`                                        | YES | `stbds_hash_string` |
| 5 | `stbds_hash_bytes`   | `size_t (void*, size_t, size_t)`                                | YES | `stbds_hash_bytes` |
| 6 | `stbds_hmfree_func`  | `void (void*, size_t)`                                          | YES | `stbds_hmfree_func` |
| 7 | `stbds_hmget_key_ts` | `void * (void*, size_t, void*, size_t, ptrdiff_t*, int)`         | YES | `stbds_hmget_key_ts` |
| 8 | `stbds_hmget_key`    | `void * (void*, size_t, void*, size_t, int)`                    | YES | `stbds_hmget_key` |
| 9 | `stbds_hmput_default`| `void * (void*, size_t)`                                        | YES | `stbds_hmput_default` |
|10 | `stbds_hmput_key`    | `void * (void*, size_t, void*, size_t, int)`                    | YES | `stbds_hmput_key` |
|11 | `stbds_shmode_func`  | `void * (size_t, int)`                                          | YES | `stbds_shmode_func` |
|12 | `stbds_hmdel_key`    | `void * (void*, size_t, void*, size_t, size_t, int)`            | YES | `stbds_hmdel_key` |
|13 | `stbds_stralloc`     | `char * (stbds_string_arena*, char*)`                           | YES | `stbds_stralloc` |
|14 | `stbds_strreset`     | `void (stbds_string_arena*)`                                    | YES | `stbds_strreset` |
|15 | `strkey`             | `char * (int)`                                                  | YES | `strkey` |
|16 | `str_put`            | `void (int)`                                                    | YES | `str_put` |

**Symbol diff (`comm -23` of the two sorted `nm -D` name lists): EMPTY.**

## `static` (internal, *not* exported) definitions — correctly not in `nm -D`

| C symbol | reason |
|----------|--------|
| `stbds_hash_seed`         | `static size_t` global (mutated by `stbds_rand_seed` / `stbds_make_hash_index`) |
| `buffer`                  | `static char buffer[256]` used by `strkey` |
| `stbds_probe_position`    | `static` |
| `stbds_log2`              | `static` |
| `stbds_make_hash_index`   | `static` |
| `stbds_siphash_bytes`     | `static` |
| `stbds_is_key_equal`      | `static` |
| `stbds_hm_find_slot`      | `static` |
| `stbds_strdup`            | `static` |

The Rust translation keeps all of the above private (no `#[no_mangle]`), so the
Rust `.so` exports exactly the same 16 names.

## Undefined (imported) symbols

C `.so` imports: `__assert_fail`, `free`, `malloc`, `memcmp`, `memcpy`,
`memmove`, `memset`, `printf`, `realloc`, `sprintf`, `strcmp`, `strlen`
(all libc) plus the usual weak `_ITM_*` / `__cxa_finalize` / `__gmon_start__`.

Rust `.so` imports the **same libc set, `__assert_fail` included**, plus Rust
runtime/`std` internals (`malloc`/`calloc`/`mmap64`/`pthread_key_*`/... ).
**0 missing/undefined non-libc symbols.**

Note: the C build has asserts **enabled** — CMake adds no `-DNDEBUG` and
`objdump -d` shows **9** distinct `__assert_fail` call sites at `lib.c` lines
401, 778, 828, 846, 849, 913, 958, 959, 960. The tenth `STBDS_ASSERT`
(`table->used_count >= 0` at line 832) is a tautology on a `size_t` and gcc
removed it. The Rust translation therefore calls glibc's `__assert_fail`
directly with the identical assertion text, `__FILE__`, line and function name,
which makes the diagnostic **byte-identical**:

```
<prog>: /…/c_src/src/lib.c:846: stbds_hmdel_key: Assertion `slot >= 0' failed.
```

`ERRORS.md` rows E25, E29, E35–E39 analyse each assert and show which are
reachable (E29 and E39 are, and are exercised in forked children).

## ABI layout parity (verified against a C `sizeof`/`offsetof` probe)

| type | C | Rust |
|------|---|------|
| `stbds_array_header` | 32 | 32 |
| `stbds_string_block` | 16 | 16 |
| `stbds_string_arena` | 24 | 24 |
| `stbds_hash_bucket`  | 128 | 128 |
| `stbds_hash_index`   | 104 | 104 |
| `offsetof(stbds_hash_index, string)`  | 72 | 72 |
| `offsetof(stbds_hash_index, storage)` | 96 | 96 |
| `struct { char *key; int value; }`    | 16 | 16 |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default (no features). Phase D still runs the full suite
under `--no-default-features` and under both `dev` and `release` profiles (the
profiles differ in `overflow-checks` / `debug-assertions`, which is a real code
path difference for a literal C translation).
