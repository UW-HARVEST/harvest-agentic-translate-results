# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Library: `stb_ds.h` implementation (`c_src/src/lib.c`, 958 lines) + the two
in-file test helpers `strkey` / `arr_ins`.

* C `.so`    : `c_src/build/libtranslated_rust.so` (cmake, GCC 11.5, no `NDEBUG`)
* Rust `.so` : `target/release/libarr_ins_lib.so` (`crate-type = ["cdylib"]`)

Generated with:

```sh
nm -D --defined-only <so> | awk '{print $3}' | sort
```

## Build-time configuration surface

| source | configurations |
|--------|----------------|
| `Cargo.toml` | **no `[features]` section at all** → exactly ONE valid combination: `--no-default-features` (== default). Verified by `grep -n "features" Cargo.toml` → no match. |
| `c_src/CMakeLists.txt` | no `option()`, no `add_definitions`, no `#ifdef`-driven variants. Single `SHARED` target from `src/lib.c`, links `m`. One configuration. |
| `c_src/src/lib.c` | all `#define`s are unconditional (`STBDS_HAS_TYPEOF`, `STBDS_HAS_LITERAL_ARRAY`, `STBDS_SIPHASH_*`, `STBDS_UNIT_TESTS` is **not** defined). `STBDS_STATS(x)` expands to nothing. No `#if`/`#ifdef` selects alternative code. One configuration. |

=> Feature-combination enumeration for Phase D is the singleton set `{ <default/empty> }`.
`stbds_siphash_bytes` is additionally guarded by
`typedef int STBDS_SIPHASH_2_4_can_only_be_used_in_64_bit_builds[sizeof(size_t)==8?1:-1];`
so the library is 64-bit only; the Rust translation hard-codes the same 64-bit
paths (`STBDS_SIZE_T_BITS == 64`).

## Defined dynamic symbols

All 16 symbols exported by the C `.so` are exported by the Rust `.so` with the
exact same name. There are no extra Rust exports and no missing ones.

| # | symbol | C signature | Rust export | status |
|---|--------|-------------|-------------|--------|
| 1 | `stbds_arrgrowf`      | `void * (void *a, size_t elemsize, size_t addlen, size_t min_cap)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 2 | `stbds_arrfreef`      | `void (void *a)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 3 | `stbds_rand_seed`     | `void (size_t seed)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 4 | `stbds_hash_string`   | `size_t (char *str, size_t seed)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 5 | `stbds_hash_bytes`    | `size_t (void *p, size_t len, size_t seed)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 6 | `stbds_hmfree_func`   | `void (void *p, size_t elemsize)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 7 | `stbds_hmget_key`     | `void * (void *a, size_t elemsize, void *key, size_t keysize, int mode)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 8 | `stbds_hmget_key_ts`  | `void * (void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 9 | `stbds_hmput_default` | `void * (void *a, size_t elemsize)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 10 | `stbds_hmput_key`    | `void * (void *a, size_t elemsize, void *key, size_t keysize, int mode)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 11 | `stbds_shmode_func`  | `void * (size_t elemsize, int mode)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 12 | `stbds_hmdel_key`    | `void * (void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 13 | `stbds_stralloc`     | `char * (stbds_string_arena *a, char *str)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 14 | `stbds_strreset`     | `void (stbds_string_arena *a)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 15 | `strkey`             | `char * (int n)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |
| 16 | `arr_ins`            | `void (int num)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` | OK |

### Symbol diff (must be empty)

```
$ comm -23 c_syms.txt rust_syms.txt   # in C, missing from Rust
<empty>
$ comm -13 c_syms.txt rust_syms.txt   # extra in Rust
<empty>
$ wc -l c_syms.txt rust_syms.txt
16 c_syms.txt
16 rust_syms.txt
```

Enforced automatically by `tests/symbol_parity.rs::c_and_rust_export_the_same_symbols`.

## Deliberately NOT exported (matches C)

| C entity | why not a dynamic symbol |
|----------|--------------------------|
| `stbds_unit_tests` | declared `extern` in the header but **never defined** in `lib.c` (`STBDS_UNIT_TESTS` is not compiled in). Not in `nm -D` of the C `.so`, so the Rust `.so` must not export it either. |
| `stbds_hash_seed` | `static size_t` → file-local. Rust: private `static mut stbds_hash_seed`. |
| `buffer` | `static char buffer[256]` → file-local. Rust: private `static mut buffer`. |
| `stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`, `stbds_strdup` | all `static` in C. Rust: private `fn`/`unsafe fn`. |

## Undefined symbols

Rust `.so` undefined list contains only libc / libgcc-unwind imports
(`malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`, `memmove`,
`memset`, `bcmp`, `strlen`, `abort`, `__errno_location`, `open64`, `read`,
`write`, `writev`, `close`, `mmap64`, `munmap`, `lseek64`, `stat64`, `fstat64`,
`statx`, `getcwd`, `getenv`, `readlink`, `realpath`, `syscall`, `gettid`,
`dl_iterate_phdr`, `pthread_key_*`, `pthread_setspecific`, `__tls_get_addr`,
`__cxa_finalize`, `__cxa_thread_atexit_impl`, `_Unwind_*`, `_ITM_*`,
`__gmon_start__`).

**0 missing / undefined non-libc symbols.** The extra imports beyond the C set
come from the Rust standard library runtime (panic machinery, std allocator,
`std::io` used by the panic printer) and are all satisfied by
`libc.so.6` / `libgcc_s.so.1`, which is confirmed by the fact that both
libraries load and every exported symbol resolves at `dlopen` time in the
integration tests (`RTLD_NOW` is what `libloading::Library::new` uses).

## Completion gate (re-verified after every change)

Driver: `./verify.sh` — enumerates the build configurations, builds both shared
objects, diffs `nm -D`, and runs the whole differential suite for every
configuration and for both Rust build profiles.

| gate | result |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 extra symbols, 0 undefined non-libc symbols in the Rust `.so` | PASS (16/16) |
| Phase B: every row of `CONFIGS.md` (79 rows) passes across randomized inputs | PASS |
| Phase C: every row of `ERRORS.md` (52 rows) has a passing error-path differential test | PASS |
| all of the above under EVERY feature combination (1: the empty/default one) | PASS |
| additionally: both Rust build profiles (`release`, `debug`) | PASS |
| 101 tests, 0 ignored, 0 failed, in each of the 2 × 1 configuration runs | PASS |

### Harness credibility (mutation check)

The differential harness was validated by injecting 12 single-token faults into
`src/lib.rs`, rebuilding, and confirming each one is detected (then restoring the
file bit-identically):

| injected fault | detected by |
|----------------|-------------|
| siphash `ROTATE_LEFT(v1,13)` → 14 | `phase_b_hash` |
| siphash `D_ROUNDS` 4 → 3 | `phase_b_hash` |
| `hash_string` `ROTATE_LEFT(hash,9)` → 8 | `phase_b_hash` |
| `stbds_hash_seed` LCG constant `0xb504f32d` → `0xb504f32c` | `phase_b_hash` |
| `strkey` prefix `"test_"` → `"tst__"` | `phase_b_hash` |
| `tombstone_count_threshold` loses the `slot_count>>4` term | `phase_b_map_binary` |
| `used_count_shrink_threshold` `>>2` → `>>1` | `phase_b_map_binary` |
| `hmput_key` bucket `index[..] = i-1` → `= i` | `phase_b_map_binary` |
| `hmdel_key` `final_index = arrlen-1-1` → `arrlen-1` | `phase_b_map_binary` |
| `arrgrowf` `min_cap < 4` → `min_cap < 8` | `phase_b_array` |
| `STBDS_STRING_ARENA_BLOCKSIZE_MIN` 512 → 256 | `phase_b_arena` |
| dropped the upper-scan `stbds_temp_key` write | `phase_c_errors` |

**12/12 caught, 0 survivors.**

### The one change made to `src/lib.rs` during verification

`stbds_stralloc` computed `STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1)`
with a plain `<<`. `a->block` is an `unsigned char`, so a caller may pass a
`block` value up to 255 and the shift count reaches 127. In C that is UB; on
x86-64 GCC emits `shl %cl`, which masks the count to 6 bits. The plain Rust
`<<` panicked under `debug_assertions` and relied on LLVM for the release case,
so it was changed to `wrapping_shl`, which masks identically and never panics.
Verified for `block` ∈ 0..27 ∪ 110..127 ∪ 128..137 ∪ 246..255 by
`err_37_stralloc_shift_out_of_range`. No other behavioural change was needed —
the translation was already byte-exact everywhere else.
