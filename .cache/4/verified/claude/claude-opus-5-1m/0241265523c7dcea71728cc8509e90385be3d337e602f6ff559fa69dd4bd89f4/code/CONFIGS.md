# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## Axes the C code actually branches on

Mechanically derived from `c_src/src/main.c` and `c_src/CMakeLists.txt`.

### Build-time axes

`c_src/CMakeLists.txt` is three effective lines: `cmake_minimum_required`,
`project(driver)`, `add_executable(driver src/main.c)`. It defines **no**
`option()`, no `target_compile_definitions`, no `#ifdef`-driven variants. The C
source contains **zero** preprocessor conditionals (`grep -c '#if\|#ifdef\|#ifndef' == 0`).

`Cargo.toml` therefore declares `[features] default = []` — an empty feature
set. The complete list of valid feature combinations is:

| # | feature combination | cargo invocation |
|---|---------------------|------------------|
| 1 | *(default, empty)* | `cargo test` |
| 2 | *(no default features — resolves to the same empty set)* | `cargo test --no-default-features` |
| 3 | *(all features — also the empty set)* | `cargo test --all-features` |

All three are exercised by `run_all_configs.sh`.

### Runtime axes

There are no runtime options: no `argv` parsing (`int main()` takes no
parameters), no environment lookups, no `setlocale` (so the C locale's
`isspace`/`isdigit`/`tolower` classifications apply), no global mode flags.
The only branches are:

* **A1 — `printLine`'s null check** (`line != NULL`): 2 states.
* **A2 — `main`'s value test** (`if (x)`): `x == 0` -> `bad()`, `x != 0` -> `good()`.
* **A3 — the `%d` conversion state machine** inside glibc's `vfscanf`, which
  `main` drives via `scanf("%d", &x)`. Its distinguishable input shapes are:
  leading-whitespace run, optional sign, the leading-`'0'` base-indication
  branch, the `TOLOWER(c) == 'x'` sub-branch (inert at base 10), the digit
  accumulation loop, the "was there a number?" check, `strtol`'s `ERANGE`
  saturation, and the final `(int)` narrowing of the `long` result.

### Input-shape axes

* `printLine`: null / empty / 1 byte / short / long / 1 MiB; ASCII vs. high
  bytes (`0x80..0xFF`) vs. control bytes; with and without embedded newlines.
* `main`/stdin: byte-stream shapes — empty, whitespace-only, whitespace prefix
  (each of the six C-locale space characters, singly and mixed), sign present /
  absent / doubled, leading zeros (0 / 1 / many), `0x` prefix, digit-run length
  (1 / 9 / 10 / 19 / 20 / 100 / 4200 digits, the last crossing the 4096-byte
  read-refill boundary), trailing garbage, trailing whitespace, multiple
  whitespace-separated tokens (only the first is consumed), magnitudes inside
  `int`, outside `int` but inside `long`, and outside `long`, and specifically
  magnitudes whose low 32 bits are zero.

## Configuration rows

Each row is checked off only after it passes across **many randomized inputs**
(fixed seed `0x5EED_1234_ABCD_EF01`, a SplitMix64 generator, see
`tests/common/mod.rs`) against **both** shared objects loaded with `libloading`.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `printLine` | A1 = non-null; empty string (`""`) | `cfg_01_print_line_empty` | [x] |
| 2 | `printLine` | A1 = non-null; single ASCII byte, all 0x01–0x7F values | `cfg_02_print_line_single_ascii` | [x] |
| 3 | `printLine` | A1 = non-null; random printable-ASCII strings, length 1..64 | `cfg_03_print_line_random_ascii` | [x] |
| 4 | `printLine` | A1 = non-null; random arbitrary non-NUL bytes 0x01..0xFF, length 1..256 (non-UTF-8 shape) | `cfg_04_print_line_random_bytes` | [x] |
| 5 | `printLine` | A1 = non-null; strings containing embedded `\n`, `\r`, `\t`, `\x0b`, `\x0c` | `cfg_05_print_line_embedded_control` | [x] |
| 6 | `printLine` | A1 = non-null; long strings at/around buffer boundaries: 1023, 1024, 1025, 4095, 4096, 4097, 65536, 1048576 bytes | `cfg_06_print_line_length_boundaries` | [x] |
| 7 | `printLine` | A1 = null | `cfg_07_print_line_null` (also `ERRORS.md` #1) | [x] |
| 8 | `bad` | no inputs; exercises `helperBad()` -> null -> `printLine` null branch | `cfg_08_bad` | [x] |
| 9 | `good` | no inputs; exercises `helperGood1()` -> static storage -> `printLine` output branch | `cfg_09_good` | [x] |
| 10 | `bad`, `good` | repeated/interleaved invocation in one process (`good;bad;good;…`, randomized order) — checks the static buffer is not consumed or mutated | `cfg_10_bad_good_interleaved` | [x] |
| 11 | `main` | A3: no whitespace prefix, no sign, single digit 0..9 (both A2 states: `0` -> `bad`, 1..9 -> `good`) | `cfg_11_main_single_digit` | [x] |
| 12 | `main` | A3: random whitespace prefix drawn from `{' ','\t','\n','\v','\f','\r'}`, length 1..8, then a random in-range integer | `cfg_12_main_whitespace_prefix` | [x] |
| 13 | `main` | A3: explicit `'+'` sign, then random magnitude in `0..=i32::MAX` | `cfg_13_main_plus_sign` | [x] |
| 14 | `main` | A3: explicit `'-'` sign, then random magnitude in `0..=2147483648` | `cfg_14_main_minus_sign` | [x] |
| 15 | `main` | A3: leading-`'0'` base-indication branch — 1..12 leading zeros then a random value (`"0"`, `"00"`, `"0007"`, …) | `cfg_15_main_leading_zeros` | [x] |
| 16 | `main` | A3: leading `'0'` followed by `x`/`X` (the inert `TOLOWER(c)=='x'` sub-branch) plus random hex-looking tail | `cfg_16_main_zero_x_prefix` | [x] |
| 17 | `main` | A3: random full-range `i32` decimal, signed and unsigned forms — the "value fits in int" shape | `cfg_17_main_random_i32` | [x] |
| 18 | `main` | A3: random `i64` magnitudes outside `int` but inside `long` — exercises the `(int) num.l` narrowing, incl. randomly forcing the low 32 bits to zero | `cfg_18_main_i64_narrowing` | [x] |
| 19 | `main` | A3: random 20..40 digit magnitudes outside `long` — exercises `strtol` `ERANGE` saturation in both signs | `cfg_19_main_erange_random` | [x] |
| 20 | `main` | A3: digit-run length boundaries 1, 9, 10, 18, 19, 20, 100, 4095, 4096, 4097, 4200 (crossing the input-refill chunk boundary), randomized digits | `cfg_20_main_digit_run_lengths` | [x] |
| 21 | `main` | A3: valid number followed by random trailing garbage (never consumed) | `cfg_21_main_trailing_garbage` | [x] |
| 22 | `main` | A3: multiple whitespace-separated tokens — only the first is converted | `cfg_22_main_multiple_tokens` | [x] |
| 23 | `main` | A3: number followed by trailing whitespace / newline / EOF-without-newline | `cfg_23_main_trailing_whitespace` | [x] |
| 24 | `main` | A3: random arbitrary byte soup (length 0..24 over the full 0x00..0xFF alphabet) — property-style fuzz over the whole state machine, incl. NUL and high bytes | `cfg_24_main_random_byte_soup` | [x] |
| 25 | `main` | A3: structured random tokens (optional whitespace, optional sign, optional zeros, random digit count, optional tail) — the cross-product of A3's sub-branches | `cfg_25_main_structured_random` | [x] |
| 26 | `main` (whole program) | end-to-end via the CMake **executable** vs. the cargo **binary**, same randomized corpus as rows 11–25, comparing stdout bytes *and* exit status | `cfg_26_executables_end_to_end` | [x] |

### Exhaustive sweeps (`tests/sweep.rs`)

The rows above are derived from the branches visible in the source. These
additional rows exist so that no *unnoticed* state transition can escape: they
enumerate byte strings exhaustively rather than sampling.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|--------------------------------------------|------|-----|
| 27 | `main` | **every** byte string of length 0–3 over `{0,1,9,+,-,SPACE,LF,x,a,NUL}` (1111 inputs) | `sweep_exhaustive_key_alphabet_len_0_to_3` | [x] |
| 28 | `main` | **every** byte string of length 4 over `{0,1,+,-,SPACE,x}` (1296 inputs) | `sweep_exhaustive_core_alphabet_len_4` | [x] |
| 29 | `main` | length-5 strings over the same alphabet, deterministic 1-in-6 stride (1296 inputs) | `sweep_exhaustive_core_alphabet_len_5_sampled` | [x] |
| 30 | `main` | 600 random strings of length 0–12 over the full `0x00..0xFF` alphabet | `sweep_random_full_byte_alphabet` | [x] |
| 31 | `main` | 600 random sign/leading-zero/1–25-digit numeric strings | `sweep_random_numeric_strings` | [x] |
| 32 | `main` | every power of two `2^0..2^70` and its ±1 neighbours, in both signs (426 inputs) — pins the `long` overflow edge and the `(int)` truncation edge at once | `sweep_powers_and_neighbours` | [x] |

### Build-configuration axis (`tests/c_opt_levels.rs`)

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference build is
unoptimized; that is the configuration every other row uses. These rows confirm
the observable contract is optimization-independent — which matters because
`helperBad()`'s undefined behavior is what gcc turns into a `NULL` return.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|--------------------------------------------|------|-----|
| 33 | `bad` | C compiled at `-O0`, `-O1`, `-O2`, `-O3`, `-Os` — asserts `helperBad()` still yields `NULL` (and hence no output) at each level | `bad_takes_the_null_branch_at_every_optimization_level` | [x] |
| 34 | `good` | same five optimization levels | `good_matches_at_every_optimization_level` | [x] |
| 35 | `printLine` | same five levels × 27 inputs incl. `NULL`, empty, and random 1–200-byte non-UTF-8 strings | `print_line_matches_at_every_optimization_level` | [x] |
| 36 | `main` | same five levels × 80 inputs spanning every `scanf` branch (EOF, matching failure, `0x`, narrowing, `ERANGE`, byte soup) | `main_matches_at_every_optimization_level` | [x] |
