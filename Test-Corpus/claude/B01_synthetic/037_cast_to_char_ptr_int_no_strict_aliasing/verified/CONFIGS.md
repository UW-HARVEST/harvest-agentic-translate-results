# CONFIGS.md — configuration-surface table (Phase B)

## Build-time configuration surface

`Cargo.toml` declares **no `[features]` section**, so the feature power-set is a
single element: the empty set.  `check_features.sh` derives this mechanically
from `Cargo.toml` (it would expand the full power-set had any feature existed)
and runs, all green:

| # | configuration | command |
|---|---------------|---------|
| B1 | no features (== `--no-default-features`) | `cargo check --offline --all-targets --no-default-features` |
| B2 | default features | `cargo check --offline --all-targets` |
| B3 | all features | `cargo check --offline --all-targets --all-features` |

`c_src/CMakeLists.txt` has no `option()`, no `add_definitions`, no `#ifdef` in
the source: one compile configuration, flag `-fno-strict-aliasing`.
The only build-time axes that remain are the Cargo profile (dev = `panic=unwind`,
release = `panic="abort"`) and the CMake default vs. PIC build; `run_all.sh`
runs the whole differential suite under **both** Cargo profiles:

| # | configuration | command |
|---|---------------|---------|
| B4 | dev profile (`panic=unwind`) cdylib + bin | `cargo test` (also with `--no-default-features`) |
| B5 | release profile (`panic="abort"`) cdylib + bin | `cargo test --release` (also with `--no-default-features`) |

## Runtime configuration axes actually branched on by the C code

Derived from `c_src/src/main.c` line by line — there is no option/flag/mode API
at all (no globals, no setters, no env lookups), so the axes are the *entry
point* and the *input shape*:

* **entry points** (all three are exercised, lowest level first):
  * `driver(int)` — dlopen'd from both `.so`s (lowest level exported symbol);
  * `main(void)` — dlopen'd from both `.so`s, run in a forked child with fd 0/1
    redirected (the low-level entry, not just the convenience wrapper);
  * the whole program — `c_src/build/driver` vs `target/{debug,release}/driver`
    as subprocesses (the end-to-end pipeline: `scanf` → `driver` → `print_hex`).
* **`driver` input shapes** (`print_hex` formats each byte with `%02x`, so the
  per-byte value classes are what the code distinguishes): bytes `< 0x10`
  (zero-padding branch), bytes `>= 0x10`, `0x00`, `0xff`, sign bit set/clear.
* **`main` stdin shapes** (what glibc's `%d` state machine distinguishes):
  whitespace-skip class, optional sign, digit run length, terminator class,
  `long` saturation, `long`→`int` truncation, descriptor type (seekable file /
  pipe / `/dev/null`), and buffer-boundary effects (glibc reads stdin in
  `BUFSIZ`-sized blocks, so inputs straddling 4096 bytes take a different
  refill path).

One row per meaningful combination the C actually treats differently.  Each row
is driven with **many randomized inputs** (SplitMix64, fixed seed `0x243F6A8885A308D3`)
unless the row is a single exact byte string.

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|-------------------------------------------|------|---|
| 1  | `driver` (.so) | `x = 0` — all four bytes `0x00`, exercises `%02x` zero-padding on every byte | `cfg_01_driver_zero` | [x] |
| 2  | `driver` (.so) | `x = -1` — all four bytes `0xff` | `cfg_02_driver_all_ones` | [x] |
| 3  | `driver` (.so) | `x` = each of `1..=255` in the low byte (`0x00000001`..`0x000000ff`): pad vs no-pad boundary at `0x0f/0x10` | `cfg_03_driver_low_byte_sweep` | [x] |
| 4  | `driver` (.so) | `x` = single bit set, all 32 positions (`1<<0 … 1<<31`) — includes the sign bit | `cfg_04_driver_single_bit` | [x] |
| 5  | `driver` (.so) | `x` = `INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`, `0x7f7f7f7f`, `0x80808080` | `cfg_05_driver_boundaries` | [x] |
| 6  | `driver` (.so) | `x` = 4 distinct byte values, each byte class mixed (`0x0a0b0c0d`, `0xf00f10ef`, …) | `cfg_06_driver_mixed_bytes` | [x] |
| 7  | `driver` (.so) | `x` = 20 000 uniformly random `i32` (seeded) | `cfg_07_driver_random_i32` | [x] |
| 8  | `driver` (.so) | `x` = random values with only bytes `< 0x10` (all-padding case) | `cfg_08_driver_random_nibbles` | [x] |
| 9  | `driver` (.so) | 4096 consecutive calls in one process, alternating values — no state leaks between calls | `cfg_29_repeated_calls` | [x] |
| 10 | `main` (.so, fd0 = file) | empty stdin (seekable, size 0) | `cfg_10_main_empty` | [x] |
| 11 | `main` (.so, fd0 = file) | digits only, no sign, no trailing newline: random `0..=INT_MAX` | `cfg_11_main_plain_digits` | [x] |
| 12 | `main` (.so, fd0 = file) | digits + trailing `"\n"`; digits + trailing `" "`; digits + trailing letter/punctuation (terminator classes) | `cfg_12_main_terminators` | [x] |
| 13 | `main` (.so, fd0 = file) | explicit `'-'` sign + random magnitude `1..=2147483648` | `cfg_13_main_negative` | [x] |
| 14 | `main` (.so, fd0 = file) | explicit `'+'` sign + random magnitude | `cfg_14_main_plus_sign` | [x] |
| 15 | `main` (.so, fd0 = file) | each leading-whitespace class alone and in random mixes (` `, `\t`, `\n`, `\v`, `\f`, `\r`) before a random value | `cfg_15_main_whitespace_classes` | [x] |
| 16 | `main` (.so, fd0 = file) | 1..64 random leading zeros before a random value (long digit run, small value) | `cfg_16_main_leading_zeros` | [x] |
| 17 | `main` (.so, fd0 = file) | digit run of random length 1..19 (fits `long`), value then truncated to `int` | `cfg_17_main_digit_run_lengths` | [x] |
| 18 | `main` (.so, fd0 = file) | digit run of random length 20..80 (forces `strtol` `ERANGE` saturation), both signs | `cfg_18_main_long_digit_runs` | [x] |
| 19 | `main` (.so, fd0 = file) | value straddling the `int` boundary: random values in `[INT_MAX-8, INT_MAX+8]` and `[INT_MIN-8, INT_MIN+8]` | `cfg_19_main_int_boundary` | [x] |
| 20 | `main` (.so, fd0 = file) | value straddling the `long` boundary: random values in `[LONG_MAX-8, LONG_MAX+8]`, `[LONG_MIN-8, LONG_MIN+8]` | `cfg_20_main_long_boundary` | [x] |
| 21 | `main` (.so, fd0 = file) | two or more numbers separated by whitespace — only the first conversion happens | `cfg_21_main_multiple_numbers` | [x] |
| 22 | `main` (.so, fd0 = file) | > 4096 bytes of leading whitespace (crosses glibc's `BUFSIZ` stdin refill) then a random value | `cfg_22_main_huge_ws_prefix` | [x] |
| 23 | `main` (.so, fd0 = file) | digit run straddling offset 4096 (random 4090..4100 leading zeros, then digits) — buffer-refill path mid-number | `cfg_23_main_number_across_buffer` | [x] |
| 24 | `main` (.so, fd0 = pipe) | non-seekable stdin (pipe): same random valid inputs as row 11 | `cfg_24_main_stdin_pipe` | [x] |
| 25 | `main` (.so, fd0 = `/dev/null`) | character device, immediate EOF | `cfg_25_main_stdin_devnull` | [x] |
| 26 | `main` (.so) | random raw byte blobs (0..64 bytes drawn from `{digits, signs, ws, letters, punctuation, NUL}`) — fuzzes the whole `%d` state machine | `cfg_26_main_random_blobs` | [x] |
| 27 | program (exe) | end-to-end `c_src/build/driver` vs `target/*/driver`: all of the above input shapes, stdin = pipe, compare stdout **and** exit status | `cfg_27_exe_end_to_end` | [x] |
| 28 | program (exe) | end-to-end with 1800 random raw byte blobs (seeded) | `cfg_28_exe_random_blobs` | [x] |
| 29 | `driver`, `main` (.so) | repeated invocations (4096 `driver` calls in one process; `main` re-invoked with fresh stdin) | `cfg_29_repeated_calls`, `cfg_29_main_repeated_invocations` | [x] |
| 30 | `driver` + `main` (.so) | both symbols resolved from the same `dlopen` handle and interleaved | `cfg_30_interleaved_symbols` | [x] |
| 31 | `main` (.so, fd0 = file) | **bytes consumed from fd 0** (glibc reads a whole `st_blksize` block; the terminating byte is pushed back, not consumed): all input shapes plus lengths 4095/4096/4097/8192/20002 and randomized tails | `cfg_31_so_main_stdin_consumption_file` | [x] |
| 32 | `main` (.so, fd0 = pipe / `/dev/null`) | same, on a non-seekable descriptor (the buffered block stays consumed) | `cfg_32_so_main_stdin_consumption_pipe` | [x] |
| 33 | `main` (.so) | **`main` called 1/2/3/5 times in one process**: the process-wide stdin buffer must be shared across calls (2nd conversion continues after the 1st pushback) and EOF must be sticky; file, pipe, `/dev/null` and closed fd 0 | `cfg_33_so_main_repeated_same_process` | [x] |
| 34 | program (exe, fd0 = file) | end-to-end, comparing stdout **and the file offset left behind** (libc's exit-time `_IO_cleanup` seek-back) | `cfg_34_exe_file_stdin_leftover` | [x] |
| 35 | program (exe, fd0 = pre-filled pipe) | end-to-end, comparing stdout **and the bytes left unread in the pipe** (no seek-back possible; sizes up to 50 000) | `cfg_35_exe_pipe_stdin_leftover` | [x] |
