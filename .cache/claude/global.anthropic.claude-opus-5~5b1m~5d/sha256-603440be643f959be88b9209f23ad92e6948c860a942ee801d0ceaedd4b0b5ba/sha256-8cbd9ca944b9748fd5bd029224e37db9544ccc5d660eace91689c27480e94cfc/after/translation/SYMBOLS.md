# SYMBOLS.md — Phase A symbol surface

C `.so`: `c_src/build/libharvest-work-<id>.so` (name derives from parent dir via
`CMakeLists.txt` `cmake_path(GET parent FILENAME project_name)`).
Rust `.so`: `translation/target/release/libsh_puts_lib.so`.

Command used:

```
diff <(nm -D --defined-only <C.so>    | awk '{print $3}' | sort) \
     <(nm -D --defined-only <RUST.so> | awk '{print $3}' | sort)
```

Result: **empty diff** — 16/16 symbols present in both, exact names.

| # | symbol | C `.so` | Rust `.so` | signature |
|---|--------|---------|------------|-----------|
| 1 | `stbds_arrgrowf`      | T | T | `void *(void *a, size_t elemsize, size_t addlen, size_t min_cap)` |
| 2 | `stbds_arrfreef`      | T | T | `void (void *a)` |
| 3 | `stbds_rand_seed`     | T | T | `void (size_t seed)` |
| 4 | `stbds_hash_string`   | T | T | `size_t (char *str, size_t seed)` |
| 5 | `stbds_hash_bytes`    | T | T | `size_t (void *p, size_t len, size_t seed)` |
| 6 | `stbds_hmfree_func`   | T | T | `void (void *p, size_t elemsize)` |
| 7 | `stbds_hmget_key_ts`  | T | T | `void *(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` |
| 8 | `stbds_hmget_key`     | T | T | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` |
| 9 | `stbds_hmput_default` | T | T | `void *(void *a, size_t elemsize)` |
| 10 | `stbds_hmput_key`    | T | T | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` |
| 11 | `stbds_shmode_func`  | T | T | `void *(size_t elemsize, int mode)` |
| 12 | `stbds_hmdel_key`    | T | T | `void *(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` |
| 13 | `stbds_stralloc`     | T | T | `char *(stbds_string_arena *a, char *str)` |
| 14 | `stbds_strreset`     | T | T | `void (stbds_string_arena *a)` |
| 15 | `strkey`             | T | T | `char *(int n)` |
| 16 | `sh_puts`            | T | T | `void (int num)` (the only symbol in `include/lib.h`) |

## Static / non-exported C entities (correctly NOT exported by Rust either)

| C entity | kind | Rust counterpart |
|---|---|---|
| `stbds_hash_seed` | `static size_t` = `0x31415926` | `static mut stbds_hash_seed` (private) |
| `stbds_probe_position` | `static` fn | private fn |
| `stbds_log2` | `static` fn | private fn |
| `stbds_make_hash_index` | `static` fn | private fn |
| `stbds_siphash_bytes` | `static` fn | private fn |
| `stbds_is_key_equal` | `static` fn | private fn |
| `stbds_hm_find_slot` | `static` fn | private fn |
| `stbds_strdup` | `static` fn | private fn |
| `buffer` | `static char[256]` | `static mut BUFFER: [u8;256]` (private) |

## Undefined (imported) symbols

The C `.so` imports `malloc free realloc memcmp memcpy memmove memset printf
sprintf strcmp strlen __assert_fail`. The Rust `.so` imports `realloc free
printf __assert_fail` plus the Rust runtime's own libc set;
`memcmp/strcmp/strlen/memmove` are re-implemented in-crate with identical
semantics (`c_strlen`, `c_strcmp_eq`, `c_memcmp_eq`, `c_memmove`, `c_memset0`).
No missing non-libc symbol.

**`__assert_fail` — a divergence found and fixed during verification.** The C
`.so` imports `__assert_fail`, i.e. the CMake build has **no `-DNDEBUG`**, so
every `STBDS_ASSERT` is *live* in C. The initial Rust translation made
`stbds_assert!` a **no-op** on the assumption that all assertions were
unreachable. That assumption is **false**: `assert(slot >= 0)` at lib.c:846 is
reachable via `stbds_hmdel_key` with `mode >= 2` on a non-tail element, where the
C aborts but the no-op Rust silently indexed `storage[-1]`. `src/lib.rs` now
calls the same `__assert_fail` with the identical assertion text, line number and
function name, so both libraries die by `SIGABRT` with the same message. See
`ERRORS.md` rows A1–A8 and the test
`phase_c_asserts.rs::x1_assert_slot_ge_zero_aborts_identically`.

## Completion gate

- [x] `nm -D` diff C vs Rust: **empty**.
- [x] 0 missing non-libc undefined symbols in the Rust `.so`.
- [x] No stubs / `unimplemented!()` anywhere in `src/lib.rs`.
