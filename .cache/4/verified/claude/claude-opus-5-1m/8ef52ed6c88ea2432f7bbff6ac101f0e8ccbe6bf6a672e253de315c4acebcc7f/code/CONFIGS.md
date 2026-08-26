# CONFIGS.md — configuration-surface table (Phase B)

## Build-time configuration axes (complete enumeration)

| axis | values the code actually distinguishes | source of truth |
|------|----------------------------------------|-----------------|
| Cargo features | **none** — `[features]` contains only `default = []` | `Cargo.toml` |
| C preprocessor configuration | **none** — `grep -c '#if\|#ifdef\|#ifndef\|#define' c_src/src/main.c` → 0 (only `#include <stdio.h>`) | `c_src/src/main.c` |
| CMake options / compile definitions | **none** — `CMakeLists.txt` is `add_executable(driver src/main.c)`; no `option()`, no `target_compile_definitions`, no `CMAKE_BUILD_TYPE` | `c_src/CMakeLists.txt` |
| C optimisation level (affects codegen of the `char` conversions) | `-O0` (the CMake default, empty `CMAKE_BUILD_TYPE`) and `-O2` | verified in rows C1/C2 |

⇒ **The complete set of feature combinations is exactly one:** the empty set.
`--no-default-features` and `--features default` (and plain `cargo test`) select
the same code. `./verify.sh` nevertheless runs the full suite under all three
spellings.

## Runtime configuration axes (derived from the C source)

The C translation unit has no options, no flags, no modes and no global state.
Its behaviour is a pure function of:

1. **which entry point** is invoked — `printHexCharLine` (the low-level one) or
   `main` (the composed pipeline that reads stdin, does the `+ 1` conversion and
   calls `printHexCharLine`);
2. **how it is invoked** — in-process `dlopen`+`dlsym` call, `dlopen`+`dlsym`
   call in a fresh process, or the linked executable itself;
3. the **argument value class** for `printHexCharLine` (`char` is *signed* on
   x86-64, and `%02x` reinterprets the promoted `int` as `unsigned`, so the sign
   of the byte selects between 2-digit and 8-digit output);
4. the **shape of stdin** for `main` (`%c` converts exactly one byte and never
   skips whitespace: empty / 1 byte / 2 bytes / many bytes, and the value of
   that first byte);
5. the **nature of the std streams** (regular file, pipe, `/dev/null`) — this
   selects glibc's buffering mode (fully buffered vs line buffered) and Rust's
   `LineWriter` behaviour.

Every meaningful combination of those axes is a row below. Every row is checked
with **many randomized inputs** (`xorshift64*`, fixed seed `0x2545F4914F6CDD1D`)
and/or **exhaustive** coverage of the 256 possible `char`/byte values, comparing
the C `.so` and the Rust `.so` byte-for-byte through their exported symbols.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|------------------------------------------|-----|
| B01 | `printHexCharLine` (in-process `dlopen`) | exhaustive: all 256 `char` bit patterns `0x00..=0xff`, stdout = regular file (fully buffered in C) | [x] |
| B02 | `printHexCharLine` (in-process `dlopen`) | 4096 randomized `char` values (seeded PRNG), stdout = regular file | [x] |
| B03 | `printHexCharLine` (in-process `dlopen`) | boundary values only: `0x00`, `0x01`, `0x7e`, `0x7f` (`CHAR_MAX`), `0x80` (`CHAR_MIN`), `0x81`, `0xfe`, `0xff` (`-1`) | [x] |
| B04 | `printHexCharLine` (in-process `dlopen`) | 4096 randomized **out-of-`char`-range `int`** arguments incl. `i32::MIN`, `i32::MAX`, `256`, `0x1ff`, `-1000` (upper 24 register bits set ⇒ exercises gcc's `movsbl` truncation) | [x] |
| B05 | `printHexCharLine` (in-process `dlopen`) | 1000 consecutive calls with randomized values in one process (no per-call state; accumulated output compared as a whole) | [x] |
| B06 | `printHexCharLine` (in-process `dlopen`) | stdout = **pipe** (still fully buffered in C, `LineWriter` in Rust), all 256 values | [x] |
| B07 | `printHexCharLine` (fresh process, `dlopen` in `examples/so_runner.rs`) | all 256 values, one process per value, stdout = regular file (flushed by `exit`) | [x] |
| B08 | `main` (fresh process, `dlopen`) | exhaustive: stdin = exactly 1 byte, for all 256 byte values | [x] |
| B09 | `main` (fresh process, `dlopen`) | stdin = 0 bytes (empty file) | [x] |
| B10 | `main` (fresh process, `dlopen`) | stdin = 2 bytes, all 256 values for the first byte, randomized second byte (only the first is converted) | [x] |
| B11 | `main` (fresh process, `dlopen`) | stdin = randomized length 3..=64 of randomized bytes, 512 cases | [x] |
| B12 | `main` (fresh process, `dlopen`) | stdin = randomized length 4 KiB..=64 KiB (crosses glibc's `BUFSIZ`/`st_blksize` and Rust's 8 KiB `BufReader`), 32 cases | [x] |
| B13 | `main` (fresh process, `dlopen`) | stdin = text with leading whitespace/newlines (`"\n\nA"`, `" A"`, `"\tA"`, `"\r\n"`) — `%c` must **not** skip it | [x] |
| B14 | `main` (fresh process, `dlopen`) | stdin = **pipe** (not seekable) carrying randomized data, 128 cases | [x] |
| B15 | `main` (fresh process, `dlopen`) | stdin = `/dev/null` | [x] |
| B16 | `main` (fresh process, `dlopen`) | stdout = **pipe** instead of a file (C: fully buffered; Rust: `LineWriter`), randomized stdin, 128 cases | [x] |
| B17 | `main` (fresh process, `dlopen`) | stdout = `/dev/null` (output invisible; only exit status compared) | [x] |
| B18 | `main` (fresh process, `dlopen`) | exit status compared for every row above (C `main` returns the constant `0`) | [x] |
| B19 | executable (`c_src/build/driver` vs `target/debug/driver`) | end-to-end process differential, exhaustive 256 single-byte stdins + 256 randomized multi-byte stdins | [x] |
| B20 | executable (release profile: `cargo build --release`, `panic = "abort"`) | end-to-end process differential, boundary + randomized stdins (the shipped artifact) | [x] |
| B21 | `main` (fresh process, `dlopen`) + both executables | stdin = **pseudo terminal** (`openpty`): `isatty(0)` is true, so glibc line-buffers `stdin` — a different `fscanf` path; canonical line discipline, incl. `\x04` (EOT ⇒ EOF) | [x] |
| C1 | both symbols | C `.so` compiled `-O0` (CMake default) — used by every row above | [x] |
| C2 | both symbols | C `.so` compiled `-O2` (tail-call to `printf`, `movsbl` truncation) — rows B01..B04, B08 repeated | [x] |
| F1 | all of the above | Cargo feature set `{}` via plain `cargo test` | [x] |
| F2 | all of the above | Cargo feature set `{}` via `cargo test --no-default-features` | [x] |
| F3 | all of the above | Cargo feature set `{}` via `cargo test --no-default-features --features default` | [x] |
| F4 | all of the above | release profile (`cargo test --release`, `panic = "abort"` for the shipped artifacts) | [x] |

## Row → test mapping

| rows | test target | test function(s) |
|------|-------------|------------------|
| B01, B02, B03, B04, B05, B06, C2 (`printHexCharLine` half) | `tests/inprocess.rs` (`harness = false`, strictly sequential — capturing fd 1 is process wide) | `B01_…`, `B02_…`, `B03_…`, `B04_err14_…`, `B05_…`, `B06_…`, `C2_…` |
| B07 … B21, C2 (`main` half) | `tests/differential.rs` | `b07_…` … `b21_…`, `c2_optimised_c_library_matches` |
| symbol parity (Phase D) | `tests/symbols.rs` | 4 tests |
| F1 … F4 | `./verify.sh` | drives the whole suite once per feature spelling + release |

## Evidence the rows are not vacuous (mutation check)

Six deliberate divergences were injected into the Rust translation one at a
time; every one was caught by this suite (and then reverted):

| mutation | first row that failed |
|----------|-----------------------|
| `{:02x}` → `{:x}` (drop the zero padding) | B19 |
| `char_hex as i32` → `(char_hex as u8) as i32` (drop the sign extension) | B11 |
| `data.wrapping_add(1)` → unsigned/masked addition | B12 |
| ignore `fscanf`'s EOF result (always take the byte) | B09 |
| FFI wrapper ignores its argument | B07 |
| skip leading whitespace like the other scanf conversions | B15 |

## Axes that do not exist in this C code (checked, not assumed)

* no runtime options/flags/modes: the public surface is `printHexCharLine(char)`
  and `main()` — no setters, no globals, no `argc`/`argv` (`int main()` declares
  no parameters; ignored-argv is covered by `errors.rs::generic_extra_argv_is_ignored`);
* no element types / widths / byte orders / counts / formats: the only data type
  is `char` and the only format string is `"%02x\n"`;
* no `#ifdef`/`option()`/feature gating anywhere (see the table at the top), so
  no `#[cfg(feature = …)]` is required in the Rust translation.
