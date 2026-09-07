# SYMBOLS.md — public symbol parity (C `.so` vs Rust `.so`)

## How the two `.so`s are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/mdcore.c src/mdmain.c)`.
`mdmain.c` contains `main()`, so the *library* surface is exactly `mdcore.c`.
`build_c.sh` (repo root) therefore builds:

* `cbuild/<OP>_<REPEAT>/libdriver_c.so` — `gcc -O2 -fPIC -shared -DOP=<OP> -DREPEAT=<N> mdcore.c`
* `cbuild/<OP>_<REPEAT>/driver_c` — the CMake `driver` executable (unmodified `CMakeLists.txt`)

Rust side: `cargo build --release --no-default-features --features <combo>` produces
`translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`) and
`translation/target/release/driver`.

## Symbol table (`nm -D --defined-only`, default config `OP=add REPEAT=5`)

| # | C symbol | type | Rust `.so` | Rust item |
|---|----------|------|------------|-----------|
| 1 | `op_add`        | `T` (func) | present | `#[no_mangle] extern "C" fn op_add` (`src/mdcore.rs`) |
| 2 | `op_sub`        | `T` (func) | present | `#[no_mangle] extern "C" fn op_sub` |
| 3 | `op_mul`        | `T` (func) | present | `#[no_mangle] extern "C" fn op_mul` |
| 4 | `helper_call`   | `T` (func) | present | `#[no_mangle] extern "C" fn helper_call` |
| 5 | `helper_ptr`    | `T` (func) | present | `#[no_mangle] extern "C" fn helper_ptr` |
| 6 | `use_generated` | `T` (func) | present | `#[no_mangle] extern "C" fn use_generated` |
| 7 | `G_OP`          | `D` (data, `int(*)(int,int)`) | present | `#[no_mangle] static G_OP: extern "C" fn(c_int,c_int)->c_int` |
| 8 | `G_OP_NAME`     | `D` (data, `const char*`)     | present | `#[no_mangle] static G_OP_NAME: CStrPtr` |

## Deliberately NOT exported (matches C)

| C entity | why not a dynamic symbol |
|----------|--------------------------|
| `accum_<OP>` (from `DEFINE_ACCUM(OP)`) | declared `static int CAT(accum_, op)(int n)` → internal linkage in C; `nm -D` shows it as a local `t`, not a dynamic symbol. Rust mirrors it with a private `fn accum(n: c_int)`. |
| `main` | lives in `mdmain.c`, which is only linked into the `driver` executable, not the `.so`. Rust mirrors it in `src/main.rs` (`[[bin]] driver`). |
| `STR/CAT/OP_FN/STEP_*/INIT_*/REP0..REP7/CHOOSE_REP/FOR_EACH/DO_LOOP/RUN_LOOP/DISPATCH_REP/DEFINE_ACCUM/ACCUM_FN` | preprocessor macros — no symbols at all. Translated into `src/mdconfig.rs` (`step`, `INIT`, `op_fn`, `REPEAT`, `run_loop`, `dispatch_rep`, `OP_NAME`, `OP_NAME_C`). |

Macro-generated symbols: the only macro that generates a *definition* is
`DEFINE_ACCUM(OP)`, and it is `static`. `OP_FN`/`ACCUM_FN`/`STEP_*`/`REP*` only
generate *uses*. So there are no macro-generated dynamic symbols to mirror.

## Undefined (imported) symbols

C `.so`: `printf@GLIBC_2.2.5` plus the usual `ld`/glibc bookkeeping
(`__cxa_finalize`, `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`,
`__gmon_start__`).

Rust `.so`: the same `printf@GLIBC_2.2.5` (see below) and the same four
bookkeeping symbols, plus what the Rust std runtime pulls in — `malloc`, `free`,
`realloc`, `calloc`, `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`,
`strlen`, `write`, `writev`, `read`, `close`, `open64`, `lseek64`, `fstat64`,
`stat64`, `statx`, `mmap64`, `munmap`, `getcwd`, `getenv`, `readlink`,
`realpath`, `abort`, `syscall`, `gettid`, `__errno_location`, `__tls_get_addr`,
`__cxa_thread_atexit_impl`, `pthread_key_*`, `pthread_setspecific`,
`dl_iterate_phdr`, `_Unwind_*`.

Every one of these is libc/libgcc. There are **0 missing/undefined non-libc
symbols** on the Rust side.

### Why the Rust `.so` imports `printf`

`mdcore.c` prints with C `printf`, and C stdio is *block*-buffered when stdout is
a pipe. A translation using `std::io::stdout()` writes straight to fd 1 instead,
so its output interleaves differently with a C consumer's own `printf` — an
observable divergence that the `examples/socall` subprocess test (`CONFIGS.md`
C50–C52) caught. `src/mdcore.rs` and `src/main.rs` therefore call libc `printf`
with the identical format strings (`c"helper.call=%d helper.acc=%d\n"`,
`c"helper.ptr=%d\n"`, `c"gen.acc=%d\n"`, `c"op=%s call=%d acc=%d g.call=%d\n"`,
`c"summary=%d\n"`), giving byte- *and* buffering-identical behaviour. `stderr`
(the `argc < 3` usage message) stays on Rust's `std::io::stderr`, which is
unbuffered exactly like C's `stderr`, and argv[0] is written as raw bytes so a
non-UTF-8 argv[0] reproduces `%s` exactly.

## Result

Verified for **all 24 CMake-representable build configurations** (`OP` × `REPEAT`
= {add,sub,mul} × {0..7}) by `run_diff.sh`, plus the **15 Cargo-only /
degenerate feature combinations** by `run_odd_combos.sh`. Both scripts diff the
sorted `nm -D` symbol-name lists of the two `.so`s.

**Symbol diff: EMPTY in all 39 configurations — 0 missing symbols.** The lists
are in fact byte-identical (`cmp` clean): the Rust `.so` exports exactly the C
`.so`'s eight symbols, no more and no fewer. Re-checked in-suite by the
`symbols_parity_nm` and `no_undefined_non_libc_symbols_in_rust_so` tests, which
also assert that the `static` `accum_<OP>` is exported by *neither* side.
