# Verification report

Differential verification of the Rust translation of `c_src/src/main.c` against
the C ground truth. Nothing in `c_src/` was modified (only `c_src/build/`, a
CMake output directory, was created).

## Completion gate

| requirement | status |
|-------------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 foreign-undefined symbols in the Rust `.so` | ✅ 5/5 C symbols exported, symbol sets are *equal* |
| Phase B: every row in `CONFIGS.md` passes across randomized inputs | ✅ 41 rows + C1 + F1/F2 |
| Phase C: every row in `ERRORS.md` has a passing error-path differential test | ✅ 18 rows + G1–G10 |
| All of the above under every feature combination | ✅ 2 combinations × 2 profiles (`./run_all.sh`: 23 PASS, 0 FAIL) |

## What the code under test is

`c_src/` contains a single translation unit (`src/main.c`, 85 lines) and a
`CMakeLists.txt` that declares only `add_executable(driver src/main.c)`. There
are no `#ifdef`s, no CMake `option()`s and no error enums, so the C side has
exactly one configuration.

To make the translation reachable through the same C ABI as the C code, the
crate now builds three things from one shared source file:

| file | role |
|------|------|
| `src/translated.rs` | the translation itself — no `#[no_mangle]` |
| `src/lib.rs` | `cdylib`: `#[no_mangle] extern "C"` wrappers for `printLine`, `printIntLine`, `bad`, `good`, `main` |
| `src/main.rs` | the `driver` executable; includes `translated.rs` via `#[path]` so it never links the library's `main` wrapper |

## How the two sides are compared

| test file | rows | mechanism |
|-----------|------|-----------|
| `tests/symbols.rs` | Phase D | `nm -D` on both `.so`s + `dlsym` of all 5 symbols, at 5 gcc optimisation levels |
| `tests/print_functions.rs` | 1–15 | `libloading` on both `.so`s **in-process**, fd 1 redirected to a temp file, byte-compared |
| `tests/subprocess_diff.rs` | 16–41, C1 | `libloading` in a **child process** (the test binary re-executes itself), comparing stdout, stderr, exit code and terminating signal |
| `tests/scanf_probe.rs` | scanf emulation | 7 476 stdin inputs against `tests/c_ref/scan_probe.c` |

Every Rust call crosses the FFI boundary through `dlsym` on `libdriver.so`, so
the `#[no_mangle]` export wrappers are themselves under test. The one exception
is `tests/scanf_probe.rs`, which has no C symbol to call (see below).

`bad()` and `good()` must run out-of-process: `bad()` is the CWE-131 defect
(`alloca(10)` then a 40-byte write). At `gcc -O1` the copy loop zeroes the saved
frame pointer at `[rbp]`, so the function returns with a corrupted `rbp` — real
undefined behaviour in the ground truth that would otherwise wreck the test
runner.

## Divergences found and fixed

1. **`SIGPIPE` disposition (real bug).** Rust's `std` installs
   `SIGPIPE = SIG_IGN` before `main`; a C program does not. With stdout on a pipe
   whose reader has closed, `c_src/build/driver` is **killed by signal 13** while
   the Rust binary exited **0**. Fixed by `restore_default_sigpipe()` in
   `src/main.rs`; regression-tested by `CONFIGS.md` row 38. The `#[no_mangle]`
   `main` wrapper deliberately does *not* do this, because the C `.so`'s `main`
   does not touch the disposition either.
2. **Vacuous test harness (real bug in the tests).** `cargo test` compiles the
   library only as an `rlib`; it never refreshes the `cdylib`. The first
   mutation-testing pass showed 4 of 6 injected bugs passing silently because the
   tests were loading a stale `libdriver.so`. `tests/common/mod.rs` now rebuilds
   the `.so`/binary itself (honouring `DRIVER_LIB_BUILD_ARGS` so the feature flags
   match) and hard-fails if the artifact predates any file in `src/`.

## Why `main`'s output cannot validate the `scanf` emulation

```c
int x = 0; scanf("%d", &x);
if (x) { good(); } else { bad(); }
```

`good()` and `bad()` both print `data[0]`, which is `0` in either branch, so the
program prints `0\n` for **every** possible stdin. The black-box corpus (rows
20–37, 1 000+ inputs) therefore proves the observable behaviour matches but says
nothing about the ~70 lines of scanf emulation. `tests/scanf_probe.rs` closes
that gap by comparing `Scanner::scan_int` against a purpose-built C reference
harness over 7 476 inputs, confirming the glibc details the translation relies
on, notably:

| stdin | glibc `%d` result | why |
|-------|-------------------|-----|
| `99999999999999999999` | `1`, `x = -1` | `strtol` saturates to `LONG_MAX`, then truncates to `int` |
| `-99999999999999999999` | `1`, `x = 0` | saturates to `LONG_MIN`, truncates to `0` |
| `4294967296` | `1`, `x = 0` | fits `long`, truncates to `int` |
| `-2147483649` | `1`, `x = 2147483647` | truncation, not clamping |
| `0x10` | `1`, `x = 0` | `%d` is base 10; conversion stops at `x` |
| `-` / `+` | `0`, `x` untouched | matching failure |
| `""` / `"   "` | `-1` (EOF), `x` untouched | EOF before any conversion |

This is the only place a Rust function is called directly rather than through
`dlsym`: `scanf` is glibc's, the emulation is internal to the translation, so
there is no export wrapper to exercise — and adding a synthetic export would
break the exact symbol-set equality that `tests/symbols.rs` asserts.

## Harness credibility: mutation testing

`./mutation_check.sh` injects 9 deliberate translation bugs and requires each to
be caught. All 9 are detected:

```
printIntLine off-by-one                                   caught by print_functions
printLine drops the trailing newline                      caught by print_functions
printLine ignores the NULL guard                          caught by print_functions
printLine lossy-converts bytes to UTF-8                   caught by print_functions
bad() prints a different element                          caught by subprocess_diff
scanf: saturate at INT_MAX instead of LONG_MAX            caught by scanf_probe
scanf: reject a leading '+'                               caught by scanf_probe
scanf: treat '\r' as a non-space                          caught by scanf_probe
executable no longer restores SIGPIPE to SIG_DFL          caught by subprocess_diff
```

## Reproducing

```sh
./run_all.sh          # C reference build + every feature combo x profile
./mutation_check.sh   # proves the harness is not vacuous
```

Randomized rows use the fixed seed `SEED = 0x5EED_1234_ABCD_9876`
(`tests/common/mod.rs`), so every run is reproducible.
