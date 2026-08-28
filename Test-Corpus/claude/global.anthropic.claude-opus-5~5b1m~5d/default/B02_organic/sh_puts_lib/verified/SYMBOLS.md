# SYMBOLS.md — Phase A: public-symbol surface

Derived mechanically from

```sh
nm -D --defined-only c_src/build/libharvest-work-L676DO.so
nm -D --defined-only translation/target/release/libsh_puts_lib.so
```

The C library is built from a single translation unit (`c_src/src/lib.c`,
`CMakeLists.txt` → `add_library(... SHARED src/lib.c)`); the public header
`c_src/include/lib.h` declares only `void sh_puts(int num);`, but every
non-`static` function in `lib.c` ends up in the dynamic symbol table.

`static` (file-local, therefore *not* exported and not required in the Rust
`.so`) helpers in the C source: `stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`, `stbds_hash_seed`, `buffer`.

## Exported symbol table

| # | symbol | C `.so` | Rust `.so` | signature (from `lib.c`) |
|---|--------|---------|------------|--------------------------|
| 1 | `stbds_arrgrowf`      | T | T | `void *(void *a, size_t elemsize, size_t addlen, size_t min_cap)` |
| 2 | `stbds_arrfreef`      | T | T | `void (void *a)` |
| 3 | `stbds_rand_seed`     | T | T | `void (size_t seed)` |
| 4 | `stbds_hash_string`   | T | T | `size_t (char *str, size_t seed)` |
| 5 | `stbds_hash_bytes`    | T | T | `size_t (void *p, size_t len, size_t seed)` |
| 6 | `stbds_hmfree_func`   | T | T | `void (void *a, size_t elemsize)` |
| 7 | `stbds_hmget_key_ts`  | T | T | `void *(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` |
| 8 | `stbds_hmget_key`     | T | T | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` |
| 9 | `stbds_hmput_default` | T | T | `void *(void *a, size_t elemsize)` |
| 10 | `stbds_hmput_key`    | T | T | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` |
| 11 | `stbds_shmode_func`  | T | T | `void *(size_t elemsize, int mode)` |
| 12 | `stbds_hmdel_key`    | T | T | `void *(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` |
| 13 | `stbds_stralloc`     | T | T | `char *(stbds_string_arena *a, char *str)` |
| 14 | `stbds_strreset`     | T | T | `void (stbds_string_arena *a)` |
| 15 | `strkey`             | T | T | `char *(int n)` |
| 16 | `sh_puts`            | T | T | `void (int num)` |

## Symbol diff

```
$ comm -23 c_syms.txt rust_syms.txt      # exported by C, missing from Rust
<empty>
$ comm -13 c_syms.txt rust_syms.txt      # exported by Rust, absent from C
<empty>
```

**0 missing symbols. 0 extra symbols.** No C source file/module was skipped by
the translation: `lib.c` is the only C source, and all 16 of its external
definitions are present as real translations (no stubs, no `unimplemented!()`).

## Undefined (imported) symbols in the Rust `.so`

`nm -D -u translation/target/release/libsh_puts_lib.so` lists only libc /
libgcc-unwind / loader symbols:

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `_Unwind_*`,
`__cxa_finalize`, `__cxa_thread_atexit_impl`, `__errno_location`,
`__gmon_start__`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`,
`dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`,
`malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
`posix_memalign`, `printf`, `pthread_key_*`, `pthread_setspecific`, `read`,
`readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`,
`write`, `writev`.

**0 missing/undefined non-libc symbols.**

## ABI-shared type layouts (must match byte-for-byte)

Callers hand these structures back and forth across the FFI boundary, and the
differential tests deliberately build a structure with the C `.so` and then
operate on it with the Rust `.so` (and vice versa), so a layout mismatch is
observable.

| type | C layout | size / align | Rust mirror |
|------|----------|--------------|-------------|
| `stbds_array_header` | `size_t length; size_t capacity; void *hash_table; ptrdiff_t temp;` | 32 / 8 | `#[repr(C)] struct stbds_array_header` |
| `stbds_string_block` | `struct stbds_string_block *next; char storage[8];` | 16 / 8 | `#[repr(C)] pub struct stbds_string_block` |
| `stbds_string_arena` | `stbds_string_block *storage; size_t remaining; unsigned char block; unsigned char mode;` | 24 / 8 | `#[repr(C)] pub struct stbds_string_arena` |
| `stbds_hash_bucket` | `size_t hash[8]; ptrdiff_t index[8];` | 128 / 8 | `#[repr(C)] struct stbds_hash_bucket` |
| `stbds_hash_index` | `char *temp_key; size_t slot_count, used_count, used_count_threshold, used_count_shrink_threshold, tombstone_count, tombstone_count_threshold, seed, slot_count_log2; stbds_string_arena string; stbds_hash_bucket *storage;` | 104 / 8 | `#[repr(C)] struct stbds_hash_index` |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default one (`--no-default-features` is
equivalent to the default here). Verified mechanically — see
`check_all_features.sh`.

## Build note that affects expected behaviour

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no `-DNDEBUG`, therefore
**`assert()` is live in the C `.so`**: any `STBDS_ASSERT` violation aborts the
process (SIGABRT) rather than returning. The Rust translation compiles
`stbds_assert!` to a no-op. Every assertion in the library is unreachable for
inputs reachable through the documented API (proved row by row in
`ERRORS.md` rows A1–A9), so this is not an observable divergence; the
error-path tests deliberately do not construct hand-corrupted internal state
that would trip a live C `assert` and take the whole test process down.
