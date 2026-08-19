# CONFIGS.md — Configuration-surface table (Phase A) + Phase B results

## Derivation

### Build-time configuration axes: exactly ONE

* `c_src/CMakeLists.txt` — no `option()`, no `add_definitions()`, no
  `target_compile_definitions()`, no `if()`. It is 3 effective lines:
  `cmake_minimum_required`, `project(driver)`, `add_executable(driver src/main.c)`.
* `c_src/src/main.c` — no `#ifdef` / `#ifndef` / `#if` anywhere
  (`grep -n "ifdef\|ifndef\|#if" c_src/src/main.c` -> no matches).
* `Cargo.toml` — no features existed; `[features] default = []` is declared
  explicitly to document that the single valid combination is the empty one.

**Enumeration of every valid feature combination: `{}` (the empty set).**
Verified with `cargo check --no-default-features` and `cargo check`
(`--all-features` is identical to both, since there are no features).
The whole Phase B/C suite is run under this one combination by
`./run_all_configs.sh`, which loops over every combination it derives from
`Cargo.toml` rather than hard-coding it.

### Runtime configuration axes (what the C actually branches on)

There are no flags, no modes, no setters, no global state to configure. Grepping
every branch in the C source (see `ERRORS.md`) shows the behaviour is a function
of exactly these axes:

| axis | values the C source distinguishes |
|---|---|
| **A. entry point** | `printLine`, `printIntLine`, `bad`, `good`, `main` (the full dynamic surface; `printLine`/`printIntLine` are the lowest-level ones and are driven directly, not only via `main`) |
| **B. `stdin` stream shape** | EOF-immediately; line terminated by `\n`; line terminated by EOF (no `\n`); line length `< 19`, `== 19`, `== 20`, `> 20` bytes (`CHAR_ARRAY_SIZE - 1` truncation boundary, remainder left in the stream); `\r\n`; embedded NUL; multiple lines (`main` performs **two** reads: `goodB2G` takes line 1, `bad` takes line 2) |
| **C. `atof` subject-sequence form** | decimal integer; decimal fraction (`.5`, `5.`); leading C whitespace (`' ' \t \v \f \r \n`); `+`/`-` sign; `e`/`E` exponent; hex float `0x…` (with/without `.`, with/without `p`-exponent, either case); `inf`/`infinity` (any case); `nan`/`nan(chars)` (any case); non-numeric -> no conversion (`0.0`); partial-parse prefixes (`1e`, `1e+`, `0x`, `0x.`, `.`, `1.2.3`, `12abc`, `--5`) |
| **D. resulting `float data` value class** | `+0.0`; `-0.0`; underflows to `0.0F` in the `double`->`float` cast; `|data| <= 1e-6` (the `goodB2G` guard boundary, incl. *exactly* `0.000001`); normal positive; normal negative; `100.0/data` inside `int` range; `100.0/data` exactly at `±2^31`; `100.0/data` outside `int` range; `data` overflows the cast to `±inf`; `data == ±inf`; `data == NaN` |
| **E. `printIntLine` argument** | `0`, `±1`, `INT_MAX`, `INT_MIN`, arbitrary `i32` |
| **F. `printLine` argument** | `NULL`; `""`; ASCII; raw non-UTF-8 bytes; text containing `%` conversion specifiers; text containing `\n`/`\t`/`\r`; 64 KiB oversized |
| **G. `main` argc/argv** | ignored by the C body: `(0, NULL)`, `(1, argv)`, `(INT_MAX, NULL)`, `(-1, NULL)` |
| **H. process-level call sequencing** | repeated calls in one process share the single `FILE *stdin` position and the single `stdout` stream: `bad` xN, `good` then `bad`, `printLine`/`printIntLine` interleaved |
| **I. `stdout` kind** | regular file (C stdio fully buffered) vs pipe (also fully buffered) — byte content must be identical either way |

`grep -c enum c_src/src/main.c` = 0, so there is no enum axis; the `int`-valued
axes (E, G) are exercised across their whole range instead.

## Configuration-surface table

Every row is driven through the exported symbols of **both** `.so` files loaded
with `libloading`, with **many randomized inputs per row** (seeded xorshift,
fixed seed `0x243F6A8885A308D3`, so runs are reproducible). Rows whose entry
point reads `stdin` are executed in a freshly `fork`/`exec`ed child process per
row, because C's `FILE *stdin` and Rust's buffered stdin both carry stream state
that must not leak between rows; within a row the calls are batched so the
shared-stream behaviour (axis H) is exercised too.

| # | entry point(s) | configuration (options set + input shape) | test fn | [x] |
|---|----------------|--------------------------------------------|---------|-----|
| C-01 | `printIntLine` | axis E: 20 000 random `i32` + all of `0, ±1, ±2, INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1`, batched in one process | `cfg_01_print_int_line_random` | [x] |
| C-02 | `printLine` | axis F: 2 000 random printable-ASCII strings, lengths 0..=257 | `cfg_02_print_line_random_ascii` | [x] |
| C-03 | `printLine` | axis F: 2 000 random **non-UTF-8** byte strings (bytes `0x01..=0xFF`, no NUL), lengths 0..=257 | `cfg_03_print_line_random_bytes` | [x] |
| C-04 | `printLine` | axis F: format-specifier payloads (`%d`, `%s`, `%n`, `%%`, `%1000000d`) and embedded `\n`/`\t`/`\r`/`\x0b`/`\x0c` | `cfg_04_print_line_format_and_ctrl` | [x] |
| C-05 | `printLine` | axis F: sizes 1 KiB / 4 KiB / 64 KiB (oversized, crosses stdio buffer boundaries) | `cfg_05_print_line_large` | [x] |
| C-06 | `printLine`, `printIntLine` | axis H: 4 000 randomly interleaved calls to both, one shared `stdout` — checks ordering/buffering | `cfg_06_interleaved_print_calls` | [x] |
| C-07 | `bad` | axes B+C+D: 600 random decimal integers (`-10^9..10^9`), each line `\n`-terminated, ≤19 bytes; batched 1 call per line | `cfg_07_bad_decimal_ints` | [x] |
| C-08 | `bad` | axes C+D: 600 random decimal fractions with 1..12 fraction digits, `\n`-terminated | `cfg_08_bad_decimal_fractions` | [x] |
| C-09 | `bad` | axis C: 600 random values with explicit `+`/`-` sign and random C-whitespace prefixes (`' ' \t \v \f \r`) | `cfg_09_bad_sign_and_whitespace` | [x] |
| C-10 | `bad` | axis C: 600 random scientific-notation values, random mantissa/`e`/`E`/sign/exponent `0..44` | `cfg_10_bad_scientific` | [x] |
| C-11 | `bad` | axis C: 600 random **hex floats** — `0x`/`0X`, 1..6 hex digits, optional `.`frac, optional `p`/`P`±exp | `cfg_11_bad_hex_floats` | [x] |
| C-12 | `bad` | axis C: all case permutations of `inf`, `infinity`, `nan`, `nan(x)`, each with no sign / `+` / `-` | `cfg_12_bad_inf_nan` | [x] |
| C-13 | `bad` | axis C: unparseable input -> `atof` returns `0.0` -> divide by zero (random letter/punctuation noise, bare `\n`, whitespace-only) | `cfg_13_bad_unparseable` | [x] |
| C-14 | `bad` | axis C: partial-parse prefixes `.`, `5.`, `.5`, `1e`, `1e+`, `1E-`, `0x`, `0X`, `0x.`, `0x1p`, `0x1p+`, `1.2.3`, `12abc`, `--5`, `+ 5`, `- 5`, `e5`, `0xg` | `cfg_14_bad_partial_parse` | [x] |
| C-15 | `bad` | axis D: `double`->`float` underflow/overflow edges `1e-45 1e-46 1e-38 1e-39 3.4e38 3.5e38 1e39 1e60 1e-60 1e308 1e309 1e-308` (± each) | `cfg_15_bad_float_cast_edges` | [x] |
| C-16 | `bad` | axis D: values making `100.0/data` land exactly on/next to `±2^31` and `±INT_MAX` (`4.656612873e-8` = 100/2^31 and its neighbours, `0x1p-24`, `0x1p-25`, `0x1p-23`, ± each) | `cfg_16_bad_int_range_edges` | [x] |
| C-17 | `bad` | axis D: 600 random `f32` bit patterns rendered as shortest round-tripping decimal (`{:e}`), covering all exponents incl. subnormals, `-0.0`, `inf` and `NaN` | `cfg_17_bad_random_float_bits` | [x] |
| C-18 | `bad` | axis B: line lengths exactly 17, 18, **19**, **20**, 21, 25, 40 bytes -> `fgets` truncation boundary, remainder left in stream, one `bad` call per fgets-chunk | `cfg_18_bad_fgets_truncation` | [x] |
| C-19 | `bad` | axis B: last line **not** `\n`-terminated (EOF-terminated), and `\r\n` endings | `cfg_19_bad_no_trailing_newline_and_crlf` | [x] |
| C-20 | `bad` | axis B: embedded NUL bytes before/inside/after the number | `cfg_20_bad_embedded_nul` | [x] |
| C-21 | `bad` | axis H: `bad` called 400 times in one process against 400 queued lines (shared stdin position + shared stdout) | `cfg_21_bad_repeated_shared_stream` | [x] |
| C-22 | `good` | axis B: EOF immediately — `goodG2B` still prints `50`, then `fgets` fails | `cfg_22_good_eof` | [x] |
| C-23 | `good` | axes C+D: 600 random values across all `atof` forms; exercises `goodG2B` (constant `50`) + `goodB2G` guard on each call | `cfg_23_good_random_values` | [x] |
| C-24 | `good` | axis D: guard boundary `fabs(data) > 0.000001` — `0.000001`, `0.0000010000001`, `9.99999e-7`, `1e-6`, `1e-7`, `-1e-6`, `-0.000001`, `0`, `-0`, `1.0000001e-6` (±) | `cfg_24_good_guard_boundary` | [x] |
| C-25 | `good` | axes C+D: `inf`/`-inf`/`nan`/`-nan`/hex/unparseable through the guard | `cfg_25_good_inf_nan_hex` | [x] |
| C-26 | `good` | axis B: `>19`-byte lines so `goodB2G` sees a truncated prefix; 1 call per chunk | `cfg_26_good_fgets_truncation` | [x] |
| C-27 | `good` | axis H: `good` called 300 times in one process against 300 queued lines | `cfg_27_good_repeated_shared_stream` | [x] |
| C-28 | `main` | axes B+G: EOF immediately (no stdin at all), `argc=1`/valid `argv` — both `fgets` calls fail | `cfg_28_main_eof` | [x] |
| C-29 | `main` | axis B: exactly ONE line — `good()` consumes it, `bad()` hits EOF | `cfg_29_main_single_line` | [x] |
| C-30 | `main` | axes B+C+D: 400 random **pairs** (line1 x line2) drawn from the full value-class generator — the real two-read pipeline | `cfg_30_main_random_line_pairs` | [x] |
| C-31 | `main` | axis B: one long `>19`-byte line so `good()` truncates and `bad()` reads the *remainder* of the same line | `cfg_31_main_truncation_carryover` | [x] |
| C-32 | `main` | axis G: `(0, NULL)`, `(1, valid argv)`, `(2, valid argv)`, `(INT_MAX, NULL)`, `(-1, NULL)`, `(INT_MIN, NULL)` | `cfg_32_main_argc_argv_variants` | [x] |
| C-33 | `main` | axis B: 300 random raw byte blobs as stdin (fuzz: random lengths, NULs, `\r`, `\n`, high bytes, no trailing newline) | `cfg_33_main_raw_byte_fuzz` | [x] |
| C-34 | `good` + `bad` | axis H: `good()` then `bad()` in one process via the two separate exports (reproduces `main`'s read order through the low-level entry points) | `cfg_34_good_then_bad_sequence` | [x] |
| C-35 | `bad` + `good` | axis H: reverse order `bad()` then `good()` — a sequence `main` never performs, so only reachable through the low-level exports | `cfg_35_bad_then_good_sequence` | [x] |
| C-36 | whole executable | end-to-end: cmake-built C `driver` vs `cargo`-built Rust `driver`, 400 random stdin blobs, stdout **and** exit status compared | `cfg_36_executable_end_to_end` | [x] |
| C-37 | whole executable | axis I: `stdout` is a **pipe** vs a **regular file**, same 60 inputs — output must be byte-identical in both cases and between C and Rust | `cfg_37_stdout_pipe_vs_file` | [x] |
| C-38 | `bad`, `good` | axis D, **double-rounding boundary**: `data` is reached as decimal -> `double` -> `float` (TWO roundings). Inputs are `f32` midpoints printed at 6..17 significant digits (which still fit the 19-byte `fgets` window) for the 14 values where `100.0/v` is an exact integer, plus 400 random `f32` midpoints | `cfg_38_bad_double_rounding_boundaries` | [x] |

### Why C-38 exists (a gap found by the mutation control, not by guessing)

`mutation_check.sh` injects deliberate divergences and asserts the suite catches
each one. Rows C-01..C-37 caught 20 of 21 mutations but were **blind** to
"`atof` parses at `f32` precision" — i.e. a translation that converted the
decimal text straight to `f32` (ONE rounding) instead of C's decimal ->
`double` -> `float` (TWO roundings). The two agree on essentially every random
input, so no amount of ordinary randomization finds it; it needs a decimal that
lands between an `f32` midpoint and the nearest `double`, which takes ~17
significant digits.

Concretely, `2.0000001192092896` (18 bytes, so it fits) is the `f32` midpoint
above `2.0` rounded to 17 digits:

* `strtod` rounds it to *exactly* the midpoint, then `(float)` rounds
  half-to-even **down** to `2.0`, so `(int)(100.0/2.0)` prints `50`;
* a single-rounding parser sees a value *above* the midpoint and rounds **up**
  to `2.0000002`, so `(int)(100.0/2.0000002)` prints `49`.

Row C-38 covers this class, and with it the mutation control reports
**21 caught / 0 missed**.

## Results

| item | value |
|---|---|
| valid feature combinations | 1 (the empty set) |
| profiles exercised | `debug` and `release` |
| `CONFIGS.md` rows | 38, all passing |
| total tests in the suite | 54 (38 `cfg_*`, 13 `err_*`, 2 harness/parity, 1 child runner) |
| mutation negative control | 21 caught / 0 missed |
| independent end-to-end spot check | 250 random stdin cases, 0 divergences |

Reproduce everything with `./run_all_configs.sh` (sweep) and
`./mutation_check.sh` (negative control).

## A second harness defect the negative control exposed

After the first sweep populated `target/debug/libdriver.so`, the mutation control
suddenly reported the `src/lib.rs` mutations (`printLine` NULL check inverted,
`printIntLine` negated, `main` returning 1) as **MISSED**. The mutations were
real; the harness was not testing them.

Cause: `cargo test --test differential` builds only the *test* target — it does
**not** rebuild the `lib`/`bin` targets. `ensure_rust_artifacts()` reused a
pre-existing `target/<profile>/libdriver.so` if one was present, so the suite
happily loaded and compared a **stale** shared object that predated the changes
in `src/`. Every test passed while verifying nothing.

Fixed by having `ensure_rust_artifacts()` rebuild the `cdylib` and `bin`
unconditionally into a dedicated `CARGO_TARGET_DIR`, plus an explicit staleness
guard that panics if either artifact's mtime is older than any file in `src/`.
With that, the control is back to **21 caught / 0 missed**.

Worth recording as a general lesson: "all tests pass" is only meaningful once you
have checked that the tests can fail.
