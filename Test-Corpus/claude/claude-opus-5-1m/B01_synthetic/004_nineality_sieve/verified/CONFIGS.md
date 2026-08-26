# CONFIGS.md — Phase B configuration surface (valid inputs)

## Public entry points

`c_src/src/main.c` is a single translation unit whose only function is
`int main(int argc, char **argv)`. There are no library functions, no options
structs and no `#ifdef`s, so the **complete** public surface is one process
invocation:

```
driver <operand>            # stdout = decimal lines, exit 0
```

`Cargo.toml` declares `[features] default = []` and no other features, and
`c_src/CMakeLists.txt` declares no build options, so there is exactly **one**
build configuration (`default` == `--no-default-features`). Both are exercised
(see `run_all.sh`).

## Axes the C code actually branches on

| axis | values the C distinguishes | where in the C |
|------|---------------------------|----------------|
| `argc` | `2` (valid) — everything else is Phase C | `if (argc != 2)` |
| leading whitespace | none, and each of `' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'`, repeated/mixed | `strtol` skips `isspace` |
| sign | absent, `'+'`, `'-'` | `strtol` |
| leading zeros | 0, 1, many | `strtol` digit loop |
| digit count / magnitude | fits `int`; outside `int` but inside `long`; outside `long` (⇒ `ERANGE` clamp to `LONG_MAX`/`LONG_MIN`) | `strtol` |
| trailing bytes | none; non-digit suffix (partial parse) | `strtol` stops at first non-digit, `end != argv[1]` |
| `long`→`int` narrowing | value in `int` range vs. out of range (truncate mod 2^32) | `int val = strtol(...)` |
| last decimal digit of `val` | `9` ⇒ break after one line; `0..8` ⇒ keep counting | `if (val % 10 == 9)` |
| sign of `val` at the `%` test | `val >= 0` (remainder `0..9`, can equal 9) vs. `val < 0` (C truncating `%` ⇒ remainder `-9..0`, **never** 9 ⇒ counts all the way up to `+9`) | `val % 10 == 9` |
| `printf("%d\n")` shape | `0`; 1..10 digits; negative sign; `INT_MIN` | `printf` |
| `val++` at `INT_MAX` | signed-overflow UB ⇒ two's-complement wrap to `INT_MIN`, loop continues | `val++` |
| stdout target | pipe, regular file, `/dev/null`, **closed** fd 1 (write fails, ignored), reader closes early (`SIGPIPE`) | `printf` return value is never checked |
| locale env | `strtol`/`printf` are locale-sensitive APIs in principle | libc |
| `argv[0]`, environment, stdin | never read ⇒ must not affect output | — |

`FULL` = whole stdout + stderr + exit status compared. `BOUND(n)` = compare the
first *n* bytes of stdout; when a run ends before *n* bytes it degenerates to a
FULL compare (EOF reached) and the exit status is compared as well. `BOUND` is
required for start values that count ~2^31 times (≈25 GB of output).

## Configuration rows

| # | entry point(s) | configuration (options set + input shape) | mode | test | [x] |
|---|----------------|-------------------------------------------|------|------|-----|
| 1 | `driver <n>` | plain positive numeral whose last digit is `9` ⇒ single line, immediate break; 512 random values in `[9, INT_MAX]` | FULL | `cfg_01_positive_ends_in_nine` | [x] |
| 2 | `driver <n>` | plain positive numeral, last digit `0..8` ⇒ 2..10 lines; 512 random values in `[0, INT_MAX-10]` | FULL | `cfg_02_positive_short_loop` | [x] |
| 3 | `driver <n>` | zero in every lexical form: `0`, `+0`, `-0`, `00000`, `  -000`, `\t+0x` | FULL | `cfg_03_zero_forms` | [x] |
| 4 | `driver <n>` | exhaustive small sweep `-300 ..= 300` (crosses zero, all last digits, both signs) | FULL | `cfg_04_exhaustive_small_range` | [x] |
| 5 | `driver <n>` | small negative start (`-2000 ..= -1`, 512 random) ⇒ negative `%` quirk, counts up through 0 to `9` | FULL | `cfg_05_small_negative` | [x] |
| 6 | `driver <n>` | negative numeral whose magnitude ends in 9 (`-9`, `-19`, `-1009`, random `-(k*10+9)`) ⇒ must **not** break early | FULL | `cfg_06_negative_ends_in_nine` | [x] |
| 7 | `driver <n>` | each whitespace prefix (`' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'`, all six mixed) × repetition 1..8 × random value | FULL | `cfg_07_whitespace_prefixes` | [x] |
| 8 | `driver <n>` | explicit `'+'` sign × 256 random values (incl. out-of-`int`) | BOUND(64 KiB) | `cfg_08_explicit_plus` | [x] |
| 9 | `driver <n>` | explicit `'-'` sign × 256 random magnitudes | BOUND(64 KiB) | `cfg_09_explicit_minus` | [x] |
| 10 | `driver <n>` | leading zeros, 1..40 of them, × random value × optional sign | BOUND(64 KiB) | `cfg_10_leading_zeros` | [x] |
| 11 | `driver <n>` | trailing garbage suffixes: alpha, `"abc"`, `"0x1f"`, `" 9"`, `"-3"`, `"+3"`, `".5"`, `"e9"`, `","`, raw `\xff`/`\x80` bytes × random values (partial parse) | BOUND(64 KiB) | `cfg_11_trailing_garbage` | [x] |
| 12 | `driver <n>` | digit-count sweep: exactly 1,2,…,10 digits (random per length), both signs | BOUND(64 KiB) | `cfg_12_digit_count_sweep` | [x] |
| 13 | `driver <n>` | `int` upper boundary: `2147483639 ..= 2147483647` — includes the `% 10 == 9` break at `…639` and the `val++` overflow wrap at `INT_MAX` | BOUND(64 KiB) | `cfg_13_int_max_boundary` | [x] |
| 14 | `driver <n>` | `int` lower boundary: `-2147483648 ..= -2147483639` (`INT_MIN` formatting, ~2^31 iterations) | BOUND(64 KiB) | `cfg_14_int_min_boundary` | [x] |
| 15 | `driver <n>` | outside `int`, inside `long`: 512 random `i64` with `|v| > INT_MAX` ⇒ narrowing mod 2^32 | BOUND(32 KiB) | `cfg_15_long_to_int_truncation` | [x] |
| 16 | `driver <n>` | powers-of-two offsets: `2^32+k`, `2^33+k`, `-(2^32)+k`, `2^31+k`, `-(2^31)-k` for `k` in `0..=12` | BOUND(32 KiB) | `cfg_16_power_of_two_offsets` | [x] |
| 17 | `driver <n>` | outside `long` ⇒ `ERANGE` clamp: `LONG_MAX`, `LONG_MAX±1`, `LONG_MIN`, `LONG_MIN±1`, and 256 random 20..40-digit numerals of both signs | BOUND(32 KiB) | `cfg_17_erange_clamp` | [x] |
| 18 | `driver <n>` | very long numerals: 100 / 1 000 / 100 000 random digits, both signs, with and without leading zeros | BOUND(32 KiB) | `cfg_18_very_long_numerals` | [x] |
| 19 | `driver <n>` | **full lexical cross-product property test**: random whitespace prefix × random sign × random leading zeros × random digit string (1..25 digits) × random suffix, 2048 fixed-seed cases | BOUND(16 KiB) | `cfg_19_random_lexical_cross_product` | [x] |
| 20 | `driver <n>` | decade sweep: for 64 random decades `N`, all `N*10 + d`, `d` in `0..=9`, both signs (isolates the `% 10` branch) | BOUND(16 KiB) | `cfg_20_decade_sweep` | [x] |
| 21 | `driver <n>` | value ending in 9 decorated with whitespace + `+`/`-` + leading zeros + garbage suffix (immediate-break path through every lexical feature at once) | FULL | `cfg_21_decorated_immediate_break` | [x] |
| 22 | `driver <n>`, stdout = regular file | random values, stdout redirected to a file (fully-buffered stdio path) — file bytes must match | FULL(file) | `cfg_22_stdout_to_file` | [x] |
| 23 | `driver <n>`, stdout = `/dev/null` | random values incl. a long run; only status/stderr observable | status | `cfg_23_stdout_devnull` | [x] |
| 24 | `driver <n>`, fd 1 **closed** | valid operand, `close(1)` before `exec` ⇒ every `printf` fails, return value unchecked ⇒ exit 0 | status | `cfg_24_stdout_closed` | [x] |
| 25 | `driver <n>`, reader closes pipe early | long-running operand; reader closes after 4 KiB ⇒ process dies from `SIGPIPE` (Rust must not have Rust's default `SIG_IGN`) | signal | `cfg_25_sigpipe_parity` | [x] |
| 26 | `driver <n>` | locale axis: `LC_ALL` unset / `C` / `POSIX` / `en_US.UTF-8` / `tr_TR.UTF-8`, and `LC_NUMERIC=de_DE.UTF-8` × random values (no digit grouping, same `isspace`) | FULL | `cfg_26_locale_variants` | [x] |
| 27 | `driver <n>` | environment axis: inherited env / completely cleared env / 64 KiB of junk env × random values | FULL | `cfg_27_environment_variants` | [x] |
| 28 | `driver <n>` | `argv[0]` axis: normal path, empty string, 4 KiB name, non-UTF-8 name (never read by the program) | FULL | `cfg_28_argv0_variants` | [x] |
| 29 | `driver <n>`, stdin varied | stdin = closed / `/dev/null` / a file with data (program reads no stdin, must consume nothing) | FULL | `cfg_29_stdin_unused` | [x] |
| 30 | `driver <n>` | operand bytes not valid UTF-8 but still parsing (`"5\xff"`, `"\t-7\x80"`, random valid numeral + random high-byte suffix) | BOUND(16 KiB) | `cfg_30_non_utf8_operands` | [x] |
| 31 | `driver <n>` | long-run tail behaviour: values whose loop crosses a 10-boundary many times (`-1..-2000`) verify the terminal `break` at `+9`; combined with rows 13–16 this covers the head **and** tail of the ~2^31-iteration runs | FULL | `cfg_31_long_run_tail` | [x] |
| 32 | `driver <n>` | 10-digit values with every possible last digit and a leading `+`/`-`/none at `INT_MAX`-adjacent magnitudes (`2147483640..2147483649`, `2147483650`) incl. the truncating one (`2147483648`, `2147483649`) | BOUND(32 KiB) | `cfg_32_ten_digit_boundary` | [x] |
| 33 | `driver <n>` | **deep stream equality**: full streams for `-10000000` (10M lines), `-1`, `2147483639`; first 256 MiB of the ~2^31-line runs `2147483647` (post-wrap), `2147483648`, `-2147483648` | streaming | `cfg_33_deep_stream_equality` | [x] |
| 34 | `driver <n>`, stdout = **pty** | glibc switches to line buffering on a terminal while the Rust port block-buffers; stream through the pty (incl. ONLCR `\n`->`\r\n`) must be identical, and the final flush must happen | FULL(pty) | `cfg_34_stdout_is_a_tty`, `cfg_34b_pty_helper_is_not_vacuous` | [x] |
| 35 | `driver <n>`, `RLIMIT_FSIZE` | write hits the file-size limit ⇒ kernel raises `SIGXFSZ`; limits 1/3/4/8/64 bytes on both the numeric and the error path — same signal, same truncated bytes (verified truncation happens exactly at the limit) | FULL(file)+signal | `cfg_35_fsize_limit_sigxfsz` | [x] |
| 36 | `driver <n>` | the exact `acc*10+digit` overflow-detection boundary, where the positive limit (`LONG_MAX`) and the negative limit (`|LONG_MIN|`, one larger) differ: last-digit sweep over `92233720368547758{79,80,81}d` × {none,`+`,`-`} × {plain, 10 leading zeros, whitespace prefix, trailing garbage}, 19-nines, 20-digit, and 100 000-space / 100 000-zero prefixed boundary values | BOUND(32 KiB) | `cfg_36_strtol_overflow_boundary` | [x] |

## Test adequacy (negative controls)

Passing tests only mean something if they can fail. `mutation_check.py` injects
10 deliberate breakages into `src/main.rs`, one at a time, rebuilds, and checks
that the suite catches each one (source is restored and hash-verified after every
mutation):

| mutation | caught by |
|----------|-----------|
| M1 `rem_euclid` instead of C's truncating `%` | `cfg_05`, `cfg_06`, `err_25` |
| M2 saturating instead of truncating `long`→`int` | `cfg_15`, `err_21`, `err_22` |
| M3 SIGPIPE disposition not restored | `cfg_25`, `err_28` |
| M4 stop instead of wrap on signed overflow | `cfg_13`, `err_26` |
| M5 error message on stderr instead of stdout | `err_07`, `err_13` |
| M6 reject trailing garbage that C accepts | `err_20`, `cfg_11` |
| M7 wrap instead of clamp on `strtol` overflow | `err_21`, `cfg_17` |
| M8 one word changed in the argc message | `err_02`, `err_03` |
| M9 `'\n'` separator changed to `' '` | `cfg_01`, `cfg_04` |
| M10 `'+'` sign rejected | `cfg_08` |

Result: **all 10 mutations caught** (`python3 mutation_check.py`).
