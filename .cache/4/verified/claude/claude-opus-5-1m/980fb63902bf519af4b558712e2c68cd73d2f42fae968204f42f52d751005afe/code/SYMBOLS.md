# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

## Sources of truth

* C `.so`: `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`,
  no `CMAKE_BUILD_TYPE` ⇒ GCC 11.5.0 at `-O0`)
* Rust `.so`: `target/debug/libupdate_md5_lib.so`
  (`[lib] name = "update_md5_lib"`, `crate-type = ["cdylib"]`)

`c_src/CMakeLists.txt` compiles exactly one translation unit: `src/lib.c`.
There are no `#ifdef` / `option()` / `target_compile_definitions` build knobs,
so there is exactly **one** C build configuration.

## Exported (defined) dynamic symbols

`nm -D --defined-only <so> | sort`

| # | C symbol | C bind/type | present in Rust `.so` | Rust bind/type | notes |
|---|----------|-------------|-----------------------|----------------|-------|
| 1 | `tflac_pack_u64le`   | `T` (global text) | ✅ yes | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |
| 2 | `tflac_md5_addsample`| `T` (global text) | ✅ yes | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |
| 3 | `update_md5`         | `T` (global text) | ✅ yes | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` (the only symbol declared in `include/lib.h`) |

**Missing from Rust `.so`: NONE.** The symbol diff is empty in the
C → Rust direction, so no export wrapper had to be added and no C source file
was left untranslated (`c_src/src/lib.c` is the whole library and all three of
its functions are translated in `src/lib.rs`).

Note that `tflac_pack_u64le` and `tflac_md5_addsample` are **not** declared in
the public header, yet the C compiler gives them external linkage (no `static`),
so they *are* part of the C `.so`'s ABI and are dlsym-able. The Rust translation
exports them too, and the differential tests call them directly through
`libloading` (Phase B/C), not only through `update_md5`.

## Symbols the Rust `.so` exports that the C `.so` does not

None that belong to the library surface. The Rust `cdylib` additionally exports
nothing beyond the three symbols above (verified with `nm -D --defined-only`);
`std`'s internals are all local/hidden.

## Undefined (imported) symbols in the Rust `.so`

`nm -D -u target/debug/libupdate_md5_lib.so` lists only C-runtime / unwinder
imports pulled in by `std`, all of which are satisfied by the platform:

`_Unwind_*@GCC_*` (11 symbols), `__errno_location`, `__tls_get_addr`, `abort`,
`bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`,
`getenv`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`,
`munmap`, `open64`, `posix_memalign`, `pthread_key_create`,
`pthread_key_delete`, `pthread_setspecific`, `read`, `readlink`, `realloc`,
`realpath`, `stat64`, `strlen`, `syscall`, `write`, `writev`, plus the weak
`_ITM_*`, `__cxa_finalize`, `__cxa_thread_atexit_impl`, `__gmon_start__`,
`gettid`, `statx`.

**0 missing / unresolvable non-libc undefined symbols.**
Verified at load time as well: the differential tests `dlopen` the Rust `.so`
(via `libloading::Library::new`, i.e. `RTLD_NOW`), which would fail outright on
any unresolved symbol.

## ABI: type and record layout parity

Confirmed by compiling a throw-away program against `c_src/include/lib.h`
(`offsetof` / `sizeof` / `_Alignof`, GCC 11.5.0, x86-64 SysV) and comparing to
the `#[repr(C)]` definitions in `src/lib.rs`:

| type | C size | C align | C field offsets | Rust size | Rust align | Rust field offsets |
|------|--------|---------|-----------------|-----------|------------|--------------------|
| `tflac_md5` | 88 | 8 | `pos`=0, `total`=8, `buffer`=16 | 88 | 8 | `pos`=0, `total`=8, `buffer`=16 |
| `tflac`     | 96 | 8 | `md5_ctx`=0, `cur_blocksize`=88, `channels`=92 | 96 | 8 | `md5_ctx`=0, `cur_blocksize`=88, `channels`=92 |

Scalar typedefs: `tflac_u8`=`u8`, `tflac_s32`=`i32`, `tflac_u32`=`u32`,
`tflac_u64`=`u64`, `tflac_uint`=`tflac_u64`=`u64` (so `sizeof(tflac_uint)`==8,
`8*sizeof(tflac_uint)`==64 and `8*sizeof(tflac_s32)`==32 — both hard-coded
constants that the Rust reproduces with `size_of`).

The layout parity is additionally asserted at runtime by
`tests/phase_b_configs.rs::layout_parity_via_ffi`, which drives the two `.so`s
across a shared raw byte arena and checks that both write/read the *same* byte
offsets (`pos`@0, `total`@8, `buffer`@16, `cur_blocksize`@88, `channels`@92, and
that the 4 padding bytes at offset 4..8 stay untouched).

## Function signatures

| symbol | C prototype | Rust `extern "C"` signature |
|--------|-------------|------------------------------|
| `tflac_pack_u64le`    | `void (tflac_u8 *d, tflac_u64 n)` | `unsafe extern "C" fn(*mut u8, u64)` |
| `tflac_md5_addsample` | `void (tflac_md5 *m, tflac_u32 bits, tflac_uint val)` | `unsafe extern "C" fn(*mut tflac_md5, u32, u64)` |
| `update_md5`          | `tflac_u32 (tflac *t, const tflac_s32 *samples)` | `unsafe extern "C" fn(*mut tflac, *const i32) -> u32` |

## Build configurations / feature combinations

`Cargo.toml` has **no `[features]` table at all**, and `grep -rn "cfg(feature"
src/` returns nothing. Therefore the complete set of valid feature combinations
is a single element:

| # | cargo invocation | status |
|---|------------------|--------|
| 1 | `cargo check --no-default-features` (≡ default, ≡ no features) | ✅ clean |

For completeness the test matrix is still run against **two** Rust `.so`
builds, because the crate does define a distinct `release` profile
(`panic = "abort"`), which is a genuinely different codegen configuration:

| # | Rust `.so` under test | how |
|---|----------------------|-----|
| 1 | `target/debug/libupdate_md5_lib.so` (opt-level 0, unwind, UB/overflow checks off per `[profile.dev]`) | `cargo build && cargo test --no-default-features` |
| 2 | `target/release/libupdate_md5_lib.so` (opt-level 3, `panic = "abort"`) | `cargo build --release` + `RUST_SO=target/release/... cargo test --no-default-features --release` |

`run_all.sh` drives both automatically.

### Why `[profile.dev]` disables `debug-assertions` / `overflow-checks`

Verification found a genuine dev-profile-only divergence: with rustc's UB checks
on, `*d.add(0) = ..` in `tflac_pack_u64le` turns a `NULL` argument into a Rust
panic → `SIGABRT` (signal 6), whereas the C faults with `SIGSEGV` (signal 11).
Release already matched C (both signal 11). Since a C library has no
language-level null check and this crate is consumed purely through its C ABI,
the checks are disabled so the `dev` `.so` behaves like `release` and like the C.
All arithmetic in `src/lib.rs` already uses explicit `wrapping_*`, so
`overflow-checks` is behaviourally irrelevant. This is captured by
`ERRORS.md` rows E1/E2/E12/E13 and the test
`err_e1_e2_e12_null_pointers_crash_identically`, which compares the actual
*signal* of a child process for each side.

## Build gotcha found during verification (important)

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` target: the
integration tests `dlopen` the `.so` instead of linking it, so cargo sees no
dependency and happily runs the suite against a **stale** `libupdate_md5_lib.so`.
This was observed live — with a stale `.so`, every one of 15 deliberately
injected bugs in `src/lib.rs` went undetected and the suite reported a clean
pass. Two safeguards are now in place:

1. `tests/harness/mod.rs::assert_not_stale()` panics if the Rust `.so` (or the C
   `.so`) is older than its sources, with instructions to rebuild.
2. `run_all.sh` always runs `cargo build` before `cargo test`.

Always use `./run_all.sh`, or `cargo build && cargo test`.

## Verification status (re-checked against the completion gate)

| gate | result |
|---|---|
| `nm -D`: 0 symbols missing from the Rust `.so` | ✅ `comm -23` diff empty, both profiles |
| `nm -D -u`: 0 unresolved non-libc symbols | ✅ (`dlopen` with `RTLD_NOW` also succeeds) |
| no C source left untranslated | ✅ `c_src/src/lib.c` is the only TU; all 3 of its functions are in `src/lib.rs` |
| no stubs / `unimplemented!()` | ✅ `grep -rn 'unimplemented\|todo!\|panic!' src/` → nothing |
| Phase B: 32/32 `CONFIGS.md` rows pass | ✅ |
| Phase C: 20/20 `ERRORS.md` rows pass | ✅ |
| Phase D: all of the above, every feature combo × dev + release | ✅ `./run_all.sh` → ALL CHECKS PASSED |
| suite has discriminating power | ✅ 19/19 genuine code mutations of `src/lib.rs` detected (negative control; 3 further mutation attempts only touched doc comments and were correctly no-ops) |
| robustness cross-check | ✅ also byte-identical against a `-O2` (`CMAKE_BUILD_TYPE=Release`) C build |

**`src/lib.rs` required no changes** — the translation was already
byte-for-byte faithful. The only edit to the crate was `Cargo.toml`
(`libloading` dev-dependency + `[profile.dev]` UB/overflow checks disabled so
the dev-profile `.so` faults on NULL the way the C does; see the comment there).
