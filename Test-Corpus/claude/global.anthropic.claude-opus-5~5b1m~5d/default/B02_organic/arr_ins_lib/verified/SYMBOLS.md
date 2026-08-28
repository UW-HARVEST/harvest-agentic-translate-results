# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Source of truth: `nm -D --defined-only` on the C shared library
(`c_src/build/libharvest-work-VR2NVk.so`) vs. the Rust cdylib
(`translation/target/release/libarr_ins_lib.so`).

Reproduce with:

```sh
./check_symbols.sh
```

## C `.so` exported (defined) symbols → Rust `.so`

All 16 symbols the C library defines are `T` (global text) in both objects.

| # | symbol | C `.so` | Rust `.so` | Rust definition |
|---|--------|---------|------------|-----------------|
| 1  | `stbds_arrgrowf`      | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_arrgrowf` |
| 2  | `stbds_arrfreef`      | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_arrfreef` |
| 3  | `stbds_rand_seed`     | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_rand_seed` |
| 4  | `stbds_hash_string`   | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_hash_string` |
| 5  | `stbds_hash_bytes`    | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_hash_bytes` |
| 6  | `stbds_hmfree_func`   | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_hmfree_func` |
| 7  | `stbds_hmget_key_ts`  | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_hmget_key_ts` |
| 8  | `stbds_hmget_key`     | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_hmget_key` |
| 9  | `stbds_hmput_default` | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_hmput_default` |
| 10 | `stbds_hmput_key`     | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_hmput_key` |
| 11 | `stbds_shmode_func`   | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_shmode_func` |
| 12 | `stbds_hmdel_key`     | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_hmdel_key` |
| 13 | `stbds_stralloc`      | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_stralloc` |
| 14 | `stbds_strreset`      | T | T | `#[no_mangle] pub unsafe extern "C" fn stbds_strreset` |
| 15 | `strkey`              | T | T | `#[no_mangle] pub unsafe extern "C" fn strkey` |
| 16 | `arr_ins`             | T | T | `#[no_mangle] pub unsafe extern "C" fn arr_ins` (the only symbol in `include/lib.h`) |

Every one of the 16 is also *called through `dlsym` on both `.so`s* by the
differential suite — the export wrappers are exercised, not just present:

| symbol | exercised by |
|--------|--------------|
| `stbds_rand_seed`     | `b06_seed_extremes`, `c41_seed_lcg_progression`, `c42_default_seed`, `e59_hash_lt_2_bumped`, `stress_reseed_midflight` |
| `stbds_hash_bytes`    | `c01`–`c06`, `e53`–`e56`, `stress_hash_long_buffers` |
| `stbds_hash_string`   | `c07`, `c08`, `e57`, `e58`, `e59`, `stress_hash_long_buffers` |
| `stbds_arrgrowf`      | `c09`–`c14`, `c48`, `e01`–`e06`, `b01`–`b03`, `stress_array_pipeline` |
| `stbds_arrfreef`      | `c13`, `e62`, `abort_arrfreef_null` |
| `stbds_hmput_default` | `c17`, `e17`–`e19`, every `DiffMap::put_default` |
| `stbds_hmput_key`     | `c18`–`c35`, `e20`–`e31`, `e64`, all stress runs |
| `stbds_hmget_key`     | `c21`, `c28`, `e10`, `e14`–`e16`, `b04`, `b05`, stress runs |
| `stbds_hmget_key_ts`  | `c22`, `e11`–`e14`, `b01`, `b05`, stress runs |
| `stbds_hmdel_key`     | `c23`–`c26`, `e32`–`e43`, `b04`, `b05`, `abort_hmdel_mode2_swap` |
| `stbds_shmode_func`   | `c29`–`c35`, `e44`, `e45`, `b02`, `b04`, `stress_map_string_shmodes` |
| `stbds_hmfree_func`   | `c36`, `e07`–`e09`, every `DiffMap::free` |
| `stbds_stralloc`      | `c37`–`c39`, `e46`–`e50`, `abort_stralloc_*` |
| `stbds_strreset`      | `c40`, `e51`, `e52`, `stress_arena_and_map_interleaved` |
| `strkey`              | `c16`, `e60`, `c33` (key source) |
| `arr_ins`             | `c15`, `e61`, `abort_harness_sanity` |

**Symbol diff (C-defined minus Rust-defined): EMPTY.**

## C symbols that are declared but never defined

These appear as `U` (undefined) in the C `.so` and are therefore *not* part of
the exported surface. They must NOT be exported by Rust either:

| symbol | why |
|--------|-----|
| `stbds_unit_tests` | `extern void stbds_unit_tests(void);` declared at `lib.c:83`, never defined |

`nm -D` on the C `.so` lists `stbds_unit_tests` only in the undefined section
(and only if referenced — it is not referenced, so it does not appear at all).
Confirmed absent from both libraries.

## C file-static (internal) symbols — intentionally not exported

Translated as private Rust `fn`/`static`, no `#[no_mangle]`:

| C static | Rust counterpart |
|----------|------------------|
| `static size_t stbds_hash_seed` | `static mut stbds_hash_seed: usize` (private) |
| `static size_t stbds_probe_position(...)` | `fn stbds_probe_position` |
| `static size_t stbds_log2(...)` | `fn stbds_log2` |
| `static stbds_hash_index *stbds_make_hash_index(...)` | `unsafe fn stbds_make_hash_index` |
| `static size_t stbds_siphash_bytes(...)` | `unsafe fn stbds_siphash_bytes` |
| `static int stbds_is_key_equal(...)` | `unsafe fn stbds_is_key_equal` |
| `static char *stbds_strdup(char *)` | `unsafe fn stbds_strdup` |
| `static char buffer[256]` | `static mut buffer: [c_char; 256]` (private) |

## Undefined (imported) symbols in the Rust `.so`

Only libc / Rust-runtime imports. There are no undefined non-libc symbols that
the C library does not also import:

* C `.so` imports: `realloc`, `free`, `memmove`, `memcpy`, `memcmp`, `strcmp`,
  `strlen`, `sprintf`, `__assert_fail`, `__stack_chk_fail` (glibc).
* Rust `.so` imports: the same allocation / string set plus the usual
  `libpthread`/`libdl`/`libgcc_s` symbols pulled in by `std`
  (`pthread_*`, `dl_iterate_phdr`, `_Unwind_*`, `__cxa_thread_atexit_impl`, ...).

All are resolvable from the standard system libraries; `ldd -r` reports no
unresolved symbol for either object (see `check_symbols.sh`).

## Notes on faithfulness relevant to symbol behaviour

* The C library is compiled by `c_src/CMakeLists.txt` **without** `NDEBUG`
  (`C_FLAGS = -fPIC` only), so `STBDS_ASSERT` == `assert` is **live**. All
  `STBDS_ASSERT` sites in the Rust translation therefore use `assert!`
  (not `debug_assert!`) so that a violated invariant aborts in both libraries.
* The Rust `[profile.release]` uses `panic = "abort"`, so a failed `assert!`
  raises `SIGABRT`, exactly like glibc's `assert`.
* All integer arithmetic that can wrap in C uses explicit `wrapping_*` in Rust,
  so a `cargo build` (debug, `overflow-checks = on`) behaves identically to
  `cargo build --release` and to the C. `run_all.sh` runs the entire suite twice
  per feature combination — once against the release cdylib, once against the
  debug cdylib — to enforce this.
* `stbds_stralloc` / `stbds_strreset` traverse the block chain with raw address
  arithmetic (`raw_load_ptr` / `raw_store_ptr`, implemented over libc `memcpy`)
  rather than `(*p).field`, because the C stores through a `realloc` result
  without a NULL check: a plain field projection on a NULL pointer trips Rust's
  *debug-profile* null/alignment UB check and aborts, where the C faults with
  SIGSEGV. With the raw form both libraries report SIGSEGV in both profiles
  (see the fatal-input parity table in `ERRORS.md`).

## Allocator-import and allocation-call parity

Both objects import exactly `malloc`, `realloc` and `free` from
`GLIBC_2.2.5`, and `tools/check_alloc_trace.sh` (an `LD_PRELOAD` interposer)
confirms the two libraries issue the **same allocator calls with the same
sizes in the same order** across six scenarios (261 calls total):

| scenario | allocator calls | result |
|----------|----------------:|--------|
| `arr` (arrgrowf growth ladder + no-op + free)        |   3 | exact |
| `arr_ins` (5 iterations of the public `arr_ins`)     |  75 | exact |
| `map_bin` (60 binary keys: grow, get, delete, free)  |  32 | exact |
| `map_strdup` (40 strdup-owned string keys + deletes) | 106 | exact |
| `map_arena` (80 arena-owned string keys)             |  23 | exact |
| `arena` (30 `stralloc`s + `strreset`)                |  22 | exact |

This required splitting the realloc helper in two so that the Rust reproduces
which of `realloc(p,n)` / `malloc(n)` the C actually emits at each site
(`lib.c:297` passes a runtime pointer, `lib.c:388/873/894/906` pass a literal
`0` that the C compiler folds to `malloc`).

## How the parity was measured

```sh
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation && ./run_all.sh   # symbols + alloc traces + all tests,
                                      # every feature combo, both cdylib profiles
```
