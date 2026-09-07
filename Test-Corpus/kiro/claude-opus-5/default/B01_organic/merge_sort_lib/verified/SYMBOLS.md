# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-4YrtAf.so`
* Rust `.so`: `translation/target/release/libmerge_sort_lib.so`

## Translation-unit inventory (source of truth)

`c_src` contains exactly one translation unit, `src/lib.c`, and one public
header, `include/lib.h`. Every function in `lib.c` other than `merge_sort` is
declared `static`, i.e. internal linkage, so it cannot appear in the dynamic
symbol table:

| C function | linkage | exported? |
|---|---|---|
| `spritebatch_internal_sprite_less_than_or_equal` | `static` | no |
| `spritebatch_internal_merge_sort_iteration` | `static` | no |
| `spritebatch_internal_merge_sort_recurse` | `static` | no |
| `merge_sort` | external | **yes** |

No module/file was skipped by the translation: `translation/src/lib.rs` contains
a counterpart for all four functions plus the `spritebatch_sprite_t` struct.

## `nm -D --defined-only` — exported symbols

| symbol | C `.so` | Rust `.so` | status |
|---|---|---|---|
| `merge_sort` | `T` @ `0x12e0` | `T` @ `0x117e0` | **match** |

Symbol diff (C exported − Rust exported): **EMPTY**.

There are no macro-generated symbols in this library (the C source contains no
function-defining macros), so there is nothing further to expand.

## `nm -D --undefined-only` — imports

C `.so` imports: `memcpy@GLIBC_2.14` plus the four standard weak
CRT/ITM/gmon markers (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`).

Rust `.so` imports the same weak markers and `memcpy@GLIBC_2.14`, plus symbols
pulled in by the Rust standard library (`libc` + `libgcc` unwinder only):
`_Unwind_*` (11), `__cxa_thread_atexit_impl`, `__errno_location`,
`__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`,
`free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`,
`memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`,
`pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`,
`readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`,
`write`, `writev`.

**Non-libc undefined symbols in the Rust `.so`: 0.** Every entry above resolves
from `libc.so.6` / `libgcc_s.so.1`, which are already part of the process image
for any C consumer. Verified by `ldd -r` reporting no unresolved symbols.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one (empty feature set). `cargo check
--no-default-features` and the default build are the same compilation. There is
therefore exactly one feature combination to verify, and `SYMBOLS.md`,
`CONFIGS.md`, and `ERRORS.md` all apply to it.

## Completion

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` relative to the C `.so`.
- [x] `nm -D` shows 0 missing/undefined **non-libc** symbols in the Rust `.so`.
- [x] No stubs / `unimplemented!()` / `todo!()` anywhere in `src/lib.rs`
      (verified by grep).
