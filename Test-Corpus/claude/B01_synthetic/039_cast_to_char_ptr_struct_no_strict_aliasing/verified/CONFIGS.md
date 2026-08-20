# CONFIGS.md — Phase B configuration surface

Every row below is exercised by a differential test that runs the **C** and the
**Rust** shared library through their exported C ABI (`libloading`) and compares
stdout **byte-for-byte** plus exit status/signal. Randomized rows use a
fixed-seed xorshift64\* PRNG, so a failure is always reproducible.

Status: **all rows pass** in the dev *and* release profiles (see the checkboxes).

## Build-time configurations (enumerated mechanically)

### Cargo features

`Cargo.toml` has **no `[features]` section** and no optional dependencies:

```
$ grep -n -A20 '\[features\]' Cargo.toml   # -> no match
```

so the feature power-set has exactly **one** element:

| # | feature combination | commands run |
|---|---------------------|--------------|
| F1 | *(empty — `--no-default-features` == default == `--all-features`)* | `cargo check --no-default-features --all-targets`, `cargo build …`, `cargo test --no-default-features`, and the same three with `--release` |

`scripts/check_all_features.sh` derives this power-set from `Cargo.toml`
mechanically (so it stays correct if features are ever added) and runs
check + build + the full test suite for each element in both profiles, plus the
plain `default` and `--all-features` invocations. Result:
`FEATURE MATRIX: all combinations pass`.

### C build-time configurations

`c_src/CMakeLists.txt` declares one target and one option
(`add_executable(driver src/main.c)`, `-fno-strict-aliasing`), and
`c_src/src/main.c` contains **no `#ifdef`/`#if`** at all — only
`#include <stdio.h>` / `<string.h>`. So there is exactly **one** C configuration.
The differential `.so` is built from the same single translation unit with the
same flag (`gcc -shared -fPIC -fno-strict-aliasing`), and `c_src/` is never
modified.

### Crate targets (all verified)

| # | target | product | note |
|---|--------|---------|------|
| T1 | `[lib] crate-type=["cdylib","rlib"]` | `libdriver.so` | loaded with `libloading` in every differential test |
| T2 | `[[bin]] driver` (`#![cfg_attr(not(test), no_main)]`) | `driver` | compared against the CMake executable `c_src/build/driver` |
| T3 | `--release` (`panic = "abort"`, optimized) | both of the above | the whole suite is re-run in this profile |
| T4 | `cargo build --all-targets` (forces libtest harnesses for lib+bin) | — | must compile; the exported `main` is `#[cfg(not(test))]` so it cannot collide with libtest's own `main` |

## Runtime configuration axes (derived from the C branches)

* **A1 — entry point**: `driver` (lowest-level export) vs `main` (reads stdin,
  then calls `driver`) vs both in one process.
* **A2 — `driver`'s `int floors`**: the only datum that varies in the output;
  every byte of its 4-byte little-endian image is printed by `print_hex`.
* **A3 — stdin byte shape for `scanf("%d")`**: leading-whitespace class (each of
  the six `isspace` bytes and mixtures, crossing newlines), sign
  (none/`+`/`-`), digit-run length, leading zeros, terminator class
  (EOF / whitespace / letter / punctuation / NUL), trailing junk, extra numbers.
* **A4 — magnitude class**: fits in `int` / fits in `long` but not `int`
  (silent truncation) / exceeds `long` (glibc `strtol` saturation).
* **A5 — stdin delivery**: pipe / regular file / `/dev/null` / write-only fd
  (`EBADF`) / directory fd (`EISDIR`) / closed fd 0 / chunked arrival / pipe held
  open without EOF.
* **A6 — stdout delivery**: pipe / regular file / `/dev/full` (`ENOSPC`) /
  pipe with a closed read end (`SIGPIPE`).
* **A7 — calls per process**: one vs many (stream/flush state).
* **A8 — process environment / `argv`**: the C never calls `setlocale` and
  `int main()` ignores `argv`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` via `.so` | `floors = 0` (all-zero image) | `config_c1_c2_c3_driver_single_calls`, `inprocess` | [x] |
| C2 | `driver` via `.so` | `floors = ±1, ±2, ±3` | same | [x] |
| C3 | `driver` via `.so` | `INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1` | same | [x] |
| C4 | `driver` via `.so` | byte patterns `0x000000ff`, `0x0000ff00`, `0x00ff0000`, `0xff000000`, `0x7f7f7f7f`, `0x80808080`, `0xdeadbeef`, `0xffffffff`, `0x01234567`, `0x89abcdef`, … (every nibble position) | `config_c4_driver_byte_patterns`, `inprocess` | [x] |
| C5 | `driver` via `.so` | all 32 powers of two, their negations and ±1 neighbours | `config_c5_driver_powers_of_two`, `inprocess` | [x] |
| C6 | `driver` via `.so` | 4096 random `i32` (seed `0xC0FFEE12345678`) + 20 000 random `i32` in-process (seed `0x111122223333`) | `config_c6_c7_driver_random_batch`, `inprocess` | [x] |
| C7 | `driver` via `.so` | A7: thousands of `driver` calls in ONE process (batch + in-process, incl. C/Rust interleaved) | same | [x] |
| C8 | `driver` via `.so` | A6: stdout is a regular file (full buffering in C) | `config_c8_driver_stdout_to_file` | [x] |
| C9 | `main` via `.so` | plain decimal, EOF terminator: `0,1,2,7,9,42,1000,123456789` | `config_c9_main_plain_decimal` | [x] |
| C10 | `main` via `.so` | sign axis × 7 magnitudes (`""`/`+`/`-`) | `config_c10_main_signs` | [x] |
| C11 | `main` via `.so` | each `isspace` byte ×{1,2,5} + mixed prefixes crossing newlines | `config_c11_main_whitespace_prefixes` | [x] |
| C12 | `main` via `.so` | 23 terminator classes × 5 numbers (EOF, `\n`, ` `, `\t`, `\r`, `\v`, `\f`, letters, `.`, `,`, `-`, `+`, `;`, `/`, `:`, NUL, `\x7f`, second number) | `config_c12_main_terminators` | [x] |
| C13 | `main` via `.so` | leading zeros: 1/18/19/20/21/40/5000 zeros, ±, then `5` or `2147483648` | `config_c13_main_leading_zeros` | [x] |
| C14 | `main` via `.so` | digit-run length 1…25 (`9`s and a rolling pattern), ± | `config_c14_main_digit_run_lengths` | [x] |
| C15 | `main` via `.so` | `int` boundaries (`±2147483645…648`, `32767/8`, `65535/6`, `255/6`) | `config_c15_main_int_boundaries` | [x] |
| C16 | `main` via `.so` | in-`long`, out-of-`int` (truncating): `2147483648`, `-2147483649`, `4294967295/6/7`, `8589934592`, `1099511627776`, `±999999999999999999` | `config_c16_main_long_not_int` | [x] |
| C17 | `main` via `.so` | `long` boundary + saturation: `9223372036854775805…809`, `-9223372036854775806…810`, `18446744073709551615/6/7`, 21/40/64/1000-digit runs ± | `config_c17_main_long_boundaries` | [x] |
| C18 | `main` via `.so` | 1500 randomized stdin strings over a pipe (random whitespace prefix, sign, 1…24 digits, leading zeros, trailing junk, 1-in-8 malformed) | `config_c18_main_random_pipe` | [x] |
| C19 | `main` via `.so` | 600 randomized stdin strings from a regular **file**, stdout to a file | `config_c19_main_random_file` | [x] |
| C20 | `main` via `.so` | A5: stdin = `/dev/null` | `config_c20_main_devnull_stdin` | [x] |
| C21 | `main` via `.so` | pipe closed with **no** terminator byte (`42`, `-`, `+`, `0`, `-0`, `2147483647`, 20-digit) | `config_c21_main_no_terminator` | [x] |
| C22 | `main` via `.so` | multi-line input, first line whitespace only (scanf crosses newlines; `fgets` would not) | `config_c22_main_multiline` | [x] |
| C23 | executable (T2) | CMake `c_src/build/driver` vs the Rust bin: 24 fixed inputs + 600 random, over pipe and file, plus `/dev/null`, `EBADF`, `EISDIR`, closed-fd-0 stdin | `config_c23_executable_corpus` | [x] |
| C24 | everything | T3: the entire suite re-run under `--release` (`panic="abort"`) | `scripts/check_all_features.sh` | [x] |
| C25 | both `.so`s | symbol config: `dlopen`+`dlsym("driver")`/`dlsym("main")` on both, `nm -D` export-set equality, `ldd` has no unresolved deps | `symbol_dlsym_both_libs`, `symbol_parity_nm_defined_only`, `symbol_no_unresolved_in_rust_so` | [x] |
| C26 | `main` via `.so` | A5: stdin pipe **held open** after the payload — must return on the terminator, not wait for EOF (10 inputs, 3 s deadline) | `config_c26_main_must_not_wait_for_eof` | [x] |
| C26b | `main` via `.so` | no terminator at all: both legitimately wait for EOF and must agree once it arrives | `config_c26b_main_waits_only_for_the_lookahead` | [x] |
| C27 | `main` via `.so` | A5: the number arrives in several `read()` chunks 25 ms apart (buffer-refill path), 6 chunk patterns | `config_c27_main_chunked_stdin` | [x] |
| C28 | `main` via `.so` + exe | A6: stdout = `/dev/full`, every `write` fails `ENOSPC` (the C ignores `printf`'s result) | `config_c28_stdout_dev_full` | [x] |
| C29 | executables | A6: stdout = pipe with closed read end → `SIGPIPE`; both must die identically (signal 13) | `config_c29_stdout_closed_pipe_sigpipe` | [x] |
| C30 | executables | A8: extra `argv` entries (`[]`, `["extra"]`, `["-x","--help"]`, `["1","2","3"]`) × 2 stdins | `config_c30_argv_ignored` | [x] |
| C31 | `main` via `.so` + exe | A8: 6 environments (`LC_ALL`/`LANG`/`LC_NUMERIC`/`LC_CTYPE`/`TZ`, incl. `de_DE.UTF-8`, `ar_SA.UTF-8`, `tr_TR.UTF-8`) × 8 inputs (grouped digits, Arabic-Indic digit, …) — the C never calls `setlocale`, so nothing may change | `config_c31_environment_independence` | [x] |
| C32 | `driver` **and** `main` | A1+A7: `driver(a)` → `main()` → `driver(b)` in ONE process, 40 randomized combinations (the composed pipeline, not isolated wrappers) | `config_c32_mixed_entry_points_one_process` | [x] |

## Harness self-check

Passing tests only prove something if they can fail. `scripts/mutation_check.py`
injects 21 behaviour-changing bugs into `src/imp.rs` (wrong `bedrooms`,
big-endian image, uppercase hex, swapped nibbles, missing newline, 15-byte
output, `isspace` without `\v` / without `\n`, `+` rejected, `+` treated as `-`,
hex digits accepted, `INT_MAX`/`LONG_MAX` saturation swaps, clamping instead of
truncation, wrapping instead of saturating accumulation, slurping stdin to EOF,
read errors not treated as EOF, `main` returning 1) and requires the suite to
fail for each; plus 2 deliberately *equivalent* mutants that must survive.
Result: `MUTATION CHECK PASSED: 23 mutants behaved as expected`.
