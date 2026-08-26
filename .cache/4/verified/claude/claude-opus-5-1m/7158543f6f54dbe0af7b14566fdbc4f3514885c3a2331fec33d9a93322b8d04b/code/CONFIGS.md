# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

## How the axes were derived

Mechanically, from the source rather than from assumptions:

```sh
$ grep -nE '#if|#ifdef|#define|#else' c_src/src/main.c      # -> no matches
$ grep -nE 'option|add_definitions|target_compile|if\(' c_src/CMakeLists.txt  # -> no matches
$ awk '/^\[features\]/{f=1;next} /^\[/{f=0} f&&NF' Cargo.toml # -> no entries
$ grep -n 'main(' c_src/src/main.c                           # -> int main()  (no argc/argv)
$ grep -n 'getenv\|setlocale\|argv' c_src/src/main.c         # -> no matches
```

Consequences, and therefore the real axes:

* **Compile-time options: none.** No `#ifdef` in the C, no `option()`/defines in
  `CMakeLists.txt`, and Cargo declares **no features** — so the only feature
  combination to verify is the empty one (`--no-default-features` ≡ default).
  The *build type* is still an axis because signed-overflow is UB in C and
  overflow checks are profile-dependent in Rust (axis **BUILD**).
* **Runtime options: none.** `main()` takes no `argc`/`argv`, reads no
  environment variable, and never calls `setlocale`, so there is no flag, mode
  or option the public API can set. The complete runtime input is therefore
  **the byte stream on `stdin`** (axis **SHAPE**) or, for the library entry
  point, **the `int` argument** (axis **ARG**).
* **Entry points (axis EP)** — the full set, lowest level first:
  * `EP-driver` — the exported `void driver(int)` symbol, called directly
    through `dlopen`/`dlsym` (the lowest-level entry point; bypasses `scanf`).
  * `EP-main-so` — the exported `int main(void)` symbol of the shared object,
    called through `dlopen`/`dlsym` by an external loader process.
  * `EP-exe` — the executable's process entry (`c_src/build/driver` vs
    `target/{debug,release}/driver`); this is the composed pipeline
    `scanf → driver → printf` end-to-end and the actual deliverable.
* **Input shapes the C actually distinguishes (axis SHAPE)** — every branch
  glibc's `%d` conversion takes: leading-whitespace skipping (each `isspace`
  byte in the `"C"` locale), the optional sign, the leading-`0` base-prefix
  probe, the digit loop, the `ungetc` of the terminator, and `strtol`'s
  `cutoff`/`cutlim` overflow clamp. Plus the value classes that make
  `2*x` / `y+=300` wrap in `driver`.

Every row is exercised with **many randomised inputs** (deterministic
xorshift64\* seeded with a fixed constant), not one hand-picked value, and both
implementations are driven through their `.so` exports / their process
boundary and compared byte-for-byte (`stdout`, `stderr`, exit status,
terminating signal).

Legend for the *configuration* column: `EP` = entry point, `SHAPE` = `stdin`
byte-stream shape, `ARG` = `int` argument, `BUILD` = build configuration.

## Table

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| 1 | `EP-driver` | ARG = 0, ±1, ±2, ±10 — the smallest values; no overflow anywhere | [x] |
| 2 | `EP-driver` | ARG = uniform random `i32` over the **full** range, 4096 samples | [x] |
| 3 | `EP-driver` | ARG = `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1` — domain extremes, `2*x` wraps | [x] |
| 4 | `EP-driver` | ARG = `±2^k` for k = 0..31 — every single-bit pattern, straddling the `2*x` overflow point | [x] |
| 5 | `EP-driver` | ARG in `[INT_MAX/2 - 200, INT_MAX/2 + 200]` — the boundary where `y += 300` wraps but `2*x` does not | [x] |
| 6 | `EP-driver` | ARG in `[INT_MIN/2 - 200, INT_MIN/2 + 200]` — the negative mirror of row 5 | [x] |
| 7 | `EP-driver` | ARG = values whose decimal output changes digit count / sign (`-150`, `-149`, `0`, `1`) — `printf("%d\n")` formatting boundaries | [x] |
| 8 | `EP-exe` | SHAPE = empty input (0 bytes) — `scanf` input failure, `x` keeps `0` | [x] |
| 9 | `EP-exe` | SHAPE = single decimal integer, no sign, no leading/trailing whitespace, randomised in `int` range | [x] |
| 10 | `EP-exe` | SHAPE = signed integer, `+` and `-` prefixes, randomised in `int` range | [x] |
| 11 | `EP-exe` | SHAPE = leading whitespace: each of `' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'` singly, and randomised mixtures (incl. multi-line, so the value is not on the first line) | [x] |
| 12 | `EP-exe` | SHAPE = randomised leading-whitespace run of 1..8200 bytes — crosses glibc's `st_blksize` buffer and Rust's `BufReader` capacity | [x] |
| 13 | `EP-exe` | SHAPE = leading zeros: 1..300 `'0'`s then a randomised value (exercises glibc's leading-`0` base-prefix probe) | [x] |
| 14 | `EP-exe` | SHAPE = digit-count sweep: exactly 1..25 digits, randomised digits — straddles `INT_MAX` (10 digits) and `LONG_MAX` (19 digits) | [x] |
| 15 | `EP-exe` | SHAPE = value in `(INT_MAX, LONG_MAX]` and `[LONG_MIN, INT_MIN)` — truncation of the `long` accumulator into `int` | [x] |
| 16 | `EP-exe` | SHAPE = value magnitude > `LONG_MAX` (20..40 random digits, and 10⁶ digits), both signs — `strtol` clamp | [x] |
| 17 | `EP-exe` | SHAPE = exact boundary literals `2147483647`, `2147483648`, `-2147483648`, `-2147483649`, `4294967295`, `4294967296`, `9223372036854775807`, `9223372036854775808`, `-9223372036854775808`, `-9223372036854775809` | [x] |
| 18 | `EP-exe` | SHAPE = trailing terminator class: EOF vs `'\n'` vs `' '` vs a letter vs punctuation vs a second number — where the digit loop stops | [x] |
| 19 | `EP-exe` | SHAPE = two or more numbers on `stdin` (only the first is consumed), randomised | [x] |
| 20 | `EP-exe` | SHAPE = randomised *arbitrary* byte soup (any of 0..255, length 0..64), 1024 samples — mixes valid/invalid without bias | [x] |
| 21 | `EP-exe` | SHAPE = randomised "almost a number" strings: sign runs, embedded spaces, `0x`/`0b` prefixes, `.`/`,`/`e` separators, unicode digits | [x] |
| 22 | `EP-exe` | SHAPE = `stdin` delivered as a **pipe** vs a **regular file** (different `st_blksize`/buffering path in glibc), same randomised payloads | [x] |
| 23 | `EP-exe` | SHAPE = randomised payload with `stdout` redirected to a **regular file** vs a **pipe** (glibc full buffering vs Rust `LineWriter`) — byte-identical file contents | [x] |
| 24 | `EP-main-so` | EP = `main` symbol resolved with `dlsym` from an external loader process; SHAPE = representative set from rows 8–21 | [x] |
| 25 | `EP-exe` | BUILD = Rust `dev` profile (overflow checks **on**) vs `release` (`panic = "abort"`), against the C default build; rows 9/14/16/17 payloads | [x] |
| 26 | `EP-exe` | BUILD = C `CMAKE_BUILD_TYPE` ∈ {default, Debug, Release, RelWithDebInfo, MinSizeRel} — the `2*x` UB must stay wrapping; overflow payloads from rows 3–6 | [x] |
| 27 | `EP-exe` | Environment: `LC_ALL`/`LC_NUMERIC` ∈ {unset, `C`, `en_US.UTF-8`} — the C never calls `setlocale`, so `isspace`/grouping must not change | [x] |
| 28 | `EP-exe` | FEATURES = the single valid Cargo feature combination (empty set: `--no-default-features` ≡ default) — all of the above | [x] |
| 29 | `EP-exe` | SHAPE = a **second reader sharing fd 0** (`{ ./driver; cat; } < file`), for a seekable file *and* a pipe, payload sizes straddling 4096/8192/50000 plus randomised trailing data — pins glibc's `st_blksize` read granularity, its single `ungetc`, and the exit-time `lseek` back to the first unconsumed byte | [x] |
| 30 | `EP-exe` | SHAPE = **consecutive runs sharing one fd 0** (`{ ./driver; ./driver; ./driver; } < "42 99 7"` ⇒ `384 498 314`), a non-zero starting offset, character devices (`/dev/zero`, `/dev/null`), and ignored `argv` | [x] |

## Row → test mapping (Phase B gate)

Every row is checked off only because the named test passes against **both**
`.so`s / executables across its randomised inputs. Re-run with `./verify.sh`.

| row(s) | test | test binary |
|--------|------|-------------|
| 1 | `row01_driver_small_values` | `tests/phase_b_ffi.rs` (libloading, in-process) |
| 2 | `row02_driver_random_full_range` | `tests/phase_b_ffi.rs` |
| 3 | `row03_driver_domain_extremes` | `tests/phase_b_ffi.rs` |
| 4 | `row04_driver_powers_of_two` | `tests/phase_b_ffi.rs` |
| 5 | `row05_driver_positive_add_overflow_band` | `tests/phase_b_ffi.rs` |
| 6 | `row06_driver_negative_overflow_band` | `tests/phase_b_ffi.rs` |
| 7 | `row07_driver_format_boundaries` | `tests/phase_b_ffi.rs` |
| 8 | `row08_exe_empty_input` | `tests/phase_b_valid.rs` |
| 9 | `row09_exe_plain_integer` | `tests/phase_b_valid.rs` |
| 10 | `row10_exe_signed_integer` | `tests/phase_b_valid.rs` |
| 11 | `row11_exe_leading_whitespace_kinds` | `tests/phase_b_valid.rs` |
| 12 | `row12_exe_long_whitespace_run` | `tests/phase_b_valid.rs` |
| 13 | `row13_exe_leading_zeros` | `tests/phase_b_valid.rs` |
| 14 | `row14_exe_digit_count_sweep` | `tests/phase_b_valid.rs` |
| 15 | `row15_exe_long_range_truncation` | `tests/phase_b_valid.rs` |
| 16 | `row16_exe_strtol_clamp` | `tests/phase_b_valid.rs` |
| 17 | `row17_exe_boundary_literals` | `tests/phase_b_valid.rs` |
| 18 | `row18_exe_terminator_classes` | `tests/phase_b_valid.rs` |
| 19 | `row19_exe_multiple_numbers` | `tests/phase_b_valid.rs` |
| 20 | `row20_exe_random_byte_soup` | `tests/phase_b_valid.rs` |
| 21 | `row21_exe_almost_numbers` | `tests/phase_b_valid.rs` |
| 22 | `row22_exe_stdin_pipe_vs_file` | `tests/phase_b_valid.rs` |
| 23 | `row23_exe_stdout_file_vs_pipe` | `tests/phase_b_valid.rs` |
| 24 | `row24_so_main_via_external_loader`, `row24b_so_driver_via_external_loader` | `tests/phase_b_valid.rs` |
| 25 | `row25_rust_dev_profile_matches`, `row25_driver_dev_profile_cdylib` | `tests/phase_b_valid.rs`, `tests/phase_b_ffi.rs` |
| 26 | `row26_c_build_types_match` | `tests/phase_b_valid.rs` |
| 27 | `row27_locale_env_is_irrelevant` | `tests/phase_b_valid.rs` |
| 28 | `row28_single_feature_combination` + the `./verify.sh` combination loop | `tests/phase_b_valid.rs` |
| 29 | `row29_exe_shared_stdin_leftovers` | `tests/phase_b_valid.rs` |
| 30 | `row30_exe_sequential_runs_and_offsets` | `tests/phase_b_valid.rs` |
