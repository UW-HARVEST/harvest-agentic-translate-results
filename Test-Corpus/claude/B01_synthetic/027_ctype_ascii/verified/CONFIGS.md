# CONFIGS.md — configuration-surface table (Phase B)

## Build-time configuration (feature combinations)

* `Cargo.toml` has **no `[features]` section** → the only valid combinations are
  the empty one. Both spellings are exercised for every phase:
  * `cargo check/test --offline` (default)
  * `cargo check/test --offline --no-default-features`
* `c_src/CMakeLists.txt` declares no `option()`, no `add_definitions`, no
  `target_compile_definitions`; `c_src/src/main.c` contains no `#ifdef`/`#if`
  → the C side has exactly one configuration too.
* Full cross product = **1 configuration**, verified by `./verify.sh`, which
  loops over both spellings.

## Runtime configuration axes (derived from the C source)

| axis | values the C code distinguishes | where in the C source |
|------|---------------------------------|------------------------|
| entry point | `driver(char)` (lowest level: the whole computation), `main(void)` (stdin → `driver`), the linked executable (end-to-end) | `main.c:29`, `main.c:48` |
| locale | `driver()` always calls `setlocale(LC_ALL, "C")`, overriding whatever locale the caller/process had | `main.c:30` |
| `char` value | the 16 distinct glibc "C"-locale class rows over `-128..=127` (see rows 1–17 below) | `main.c:32-45` (12 `is*` macros + `tolower`/`toupper` table lookups) |
| how the value crosses the FFI boundary | `char` argument in a register: `i8` from a `void driver(char)` prototype, or an `int` whose low byte is the argument | `main.c:29` |
| stdin shape | first byte only is consumed; regular file (seekable) / pipe (non-seekable) / empty / larger than the stdio buffer | `main.c:49` `getchar()` |
| stdout shape | regular file vs pipe (buffering / flush granularity) | 14 × `printf` |
| call multiplicity | `driver` (and `main`) may be called more than once per process — `setlocale` re-run, stdio state reused | no guard in `main.c` |

Every row below is checked with **many randomised inputs** (deterministic
xorshift64\* PRNG, fixed seed `0x243F6A8885A308D3`, see `tests/common/mod.rs`)
and compared byte-for-byte between the C `.so` and the Rust `.so` loaded through
`libloading`.

## Rows

| #  | entry point(s) | configuration (options set + input shape) | test | done |
|----|----------------|-------------------------------------------|------|------|
| 1  | `driver` | `0x00..=0x08` — control chars with no other class bit (exhaustive + random order) | `cfg_01_cntrl_plain` | [x] |
| 2  | `driver` | `0x09` TAB — `cntrl+space+blank`, the only blank control char | `cfg_02_tab` | [x] |
| 3  | `driver` | `0x0A..=0x0D` — `cntrl+space`, not blank | `cfg_03_cntrl_space` | [x] |
| 4  | `driver` | `0x0E..=0x1F` — control chars, upper end of the range | `cfg_04_cntrl_upper` | [x] |
| 5  | `driver` | `0x20` SPACE — `space+blank+print` but **not** graph | `cfg_05_space` | [x] |
| 6  | `driver` | `0x21..=0x2F` — punctuation block before the digits | `cfg_06_punct_low` | [x] |
| 7  | `driver` | `0x30..=0x39` — digits: `alnum+digit+xdigit+print+graph` | `cfg_07_digits` | [x] |
| 8  | `driver` | `0x3A..=0x40` — punctuation block between digits and `A` | `cfg_08_punct_mid` | [x] |
| 9  | `driver` | `0x41..=0x46` — `A`–`F`: uppercase **and** hex digits | `cfg_09_upper_hex` | [x] |
| 10 | `driver` | `0x47..=0x5A` — `G`–`Z`: uppercase, not hex | `cfg_10_upper_nonhex` | [x] |
| 11 | `driver` | `0x5B..=0x60` — punctuation block between `Z` and `a` | `cfg_11_punct_high` | [x] |
| 12 | `driver` | `0x61..=0x66` — `a`–`f`: lowercase **and** hex digits | `cfg_12_lower_hex` | [x] |
| 13 | `driver` | `0x67..=0x7A` — `g`–`z`: lowercase, not hex | `cfg_13_lower_nonhex` | [x] |
| 14 | `driver` | `0x7B..=0x7E` — punctuation block after `z` | `cfg_14_punct_top` | [x] |
| 15 | `driver` | `0x7F` DEL — control char above the printable range | `cfg_15_del` | [x] |
| 16 | `driver` | `0x80..=0xFF` — negative `char` values (glibc's negative table half): random sample of 200 draws | `cfg_16_negative_chars_random` | [x] |
| 17 | `driver` | exhaustive sweep over **all 256** `char` values, in randomised order | `cfg_17_exhaustive_all_chars` | [x] |
| 18 | `driver` | value crosses the FFI boundary as `int` (`void driver(int)` prototype) with random high bytes — only the low byte may be significant | `cfg_18_ffi_int_low_byte_only` | [x] |
| 19 | `driver` | **many calls in one process**, random 64-value sequences → outputs must concatenate identically (locale reset each call, no state drift) | `cfg_19_many_calls_one_process` | [x] |
| 20 | `driver` | caller's locale switched to `C.utf8` / `en_US.utf8` / `en_US.iso88591` (a locale in which `0x80..0xFF` *are* alphabetic) before the call, random bytes each | `cfg_20_host_locale_variants` | [x] |
| 21 | `driver` | stdout is a **pipe** instead of a regular file (different stdio buffering path), random bytes | `cfg_21_stdout_is_pipe` | [x] |
| 22 | `main`   | stdin = regular file with exactly 1 byte, exhaustive over all 256 byte values | `cfg_22_main_single_byte_all` | [x] |
| 23 | `main`   | stdin = regular file with 2..4096 random bytes (only the first is consumed), 64 random buffers | `cfg_23_main_multibyte_file` | [x] |
| 24 | `main`   | stdin = regular file larger than the stdio buffer (8 KiB + random extra), random content | `cfg_24_main_large_file` | [x] |
| 25 | `main`   | stdin = **pipe** (non-seekable) with random content, 64 draws | `cfg_25_main_stdin_pipe` | [x] |
| 26 | `main`   | stdin = pipe, stdout = pipe (both non-seekable), random content | `cfg_26_main_both_pipes` | [x] |
| 27 | `main`   | stdin empty (EOF) with stdout = file and stdout = pipe | `cfg_27_main_eof_both_stdout_shapes` | [x] |
| 28 | `main`   | caller's locale switched to `en_US.iso88591` before `main`, random first byte ≥ `0x80` | `cfg_28_main_host_locale_latin1` | [x] |
| 29 | `main`   | `main` called twice in one process on the same stdin (stdio buffer must hand out successive bytes) | `cfg_29_main_twice_same_process` | [x] |
| 30 | `main` + `driver` | mixed sequence in one process: `main` (consumes a byte) followed by `driver` calls with random values | `cfg_30_main_then_driver_mixed` | [x] |
| 31 | executable | end-to-end `c_src/build/driver` vs `target/debug/driver`: exhaustive all 256 single-byte inputs + EOF, comparing stdout **and** exit status | `binaries_e2e.rs::e2e_all_bytes_and_eof` | [x] |
| 32 | executable | end-to-end with multi-byte / large stdin, stdout to a pipe, random content | `binaries_e2e.rs::e2e_random_multibyte_stdin_pipe_stdout` | [x] |
| 33 | executable | end-to-end with `LC_ALL=en_US.iso88591` / `C.utf8` in the environment (the program forces `"C"`), random bytes | `binaries_e2e.rs::e2e_env_locale_variants` | [x] |
| 34 | executable | end-to-end with stdin = `/dev/null` and stdin = a directory fd, and with stdout discarded | `binaries_e2e.rs::e2e_all_bytes_and_eof`, `e2e_stdin_is_a_directory`, `e2e_stdout_closed` | [x] |
| 35 | executable | stderr must stay empty for random inputs (neither implementation may emit diagnostics) | `binaries_e2e.rs::e2e_no_stderr_output` | [x] |

## Profiles

The same 35 rows are run under `--profile dev` **and** `--profile release`
(`panic = "abort"`, optimisations on), because `[profile.release]` in
`Cargo.toml` changes code generation for the `cdylib`. `PROFILES="debug release"
./verify.sh` covers all 4 (2 feature spellings × 2 profiles) configurations.
