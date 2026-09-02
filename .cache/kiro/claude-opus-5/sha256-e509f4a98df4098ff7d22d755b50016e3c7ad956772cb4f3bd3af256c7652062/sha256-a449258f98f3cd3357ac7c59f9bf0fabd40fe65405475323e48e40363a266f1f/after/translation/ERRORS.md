# Differential verification of the Rust translation

Reference: `c_src/src/main.c`, built with CMake (`c_src/build/driver`).
Candidate: `translation/src/main.rs` (`translation/target/release/driver`).

Comparison method: both programs are run as subprocesses with identical stdin;
stdout, stderr and exit status are compared byte for byte
(`translation/tests/differential.rs`). Nothing in `c_src/` was modified.

## Mismatches found and their causes

### 1. Panic instead of silent failure when stdout cannot be written

- **Symptom.** With stdout pointed at `/dev/full`:
  - C: exit status `0`, empty stderr.
  - Rust: exit status `134` (`SIGABRT`, because the release profile sets
    `panic = "abort"`), plus a panic message on stderr:
    `failed printing to stdout: No space left on device (os error 28)`.
- **Cause.** `main.rs` implemented `printLine`/`printIntLine` with `println!`.
  `println!` unwraps the write result and panics on failure. C's `printf`
  signals errors only through its return value, which `printLine` and
  `printIntLine` discard, so the C program keeps going and `main` still
  `return 0`s. The final flush inside `exit` also discards its error.
- **Fix.** `print_line`/`print_int_line` now delegate to `cio::printf_line` and
  `cio::printf_int_line`, which write through `write_all` and drop the `Result`.
  `cio::flush()` is called before `std::process::exit(0)` and its error is
  discarded the same way. `cio.rs` already existed in the crate with exactly
  these semantics but was not wired into `main.rs` (the module was not even
  declared, so it was never compiled).
- **Regression test.** `unwritable_stdout_does_not_change_exit_status`. Reverting
  `print_line` to `println!` makes it fail with `C Some(0) vs Rust Some(101)`.

No other mismatch was found. Everything below was checked and already agreed.

## Behaviour that had to be preserved rather than fixed

These are the places where the C does something surprising; the Rust matches it
and must keep matching it.

- **Divide by zero in `bad()`.** `bad()` has no guard, so a `data` of `0.0F`
  makes `100.0 / data` infinite. The subsequent `(int)` cast is undefined
  behaviour in C; the reference build compiles it to `cvttsd2si`, which yields
  the "integer indefinite" value `INT_MIN`. `cruntime::f64_to_int` returns
  `i32::MIN` for NaN and for anything outside `int` range so the printed
  `-2147483648` matches. This also covers merely out-of-range quotients such as
  `100.0 / 1e-30`.
- **NaN makes both comparisons false.** `nan` input leaves
  `fabs(data) > 0.000001` false in `goodB2G()` (so it prints the divide-by-zero
  message) while `bad()` prints `INT_MIN`. Rust's `f64::abs` comparison and
  `f64_to_int` reproduce both.
- **`-0`.** `fabs(-0.0) > 0.000001` is false, so `goodB2G()` reports a divide by
  zero; `bad()` computes `-inf` and still prints `-2147483648`.
- **Reading order.** `main` calls `good()` before `bad()`, and inside `good()`
  `goodG2B()` reads nothing while `goodB2G()` performs the *first* `fgets`.
  `bad()` performs the second. So line 1 drives the `goodB2G` output and line 2
  drives the `bad` output.
- **`fgets`, not `scanf`.** `CHAR_ARRAY_SIZE` is 20, so each call stores at most
  19 bytes and stops immediately after a newline without reading past it. A line
  of 19 or more payload bytes is therefore *split*: `goodB2G` gets the first 19
  bytes and `bad` gets the remainder (which may be just `"\n"`, i.e. `0.0`).
  `cruntime::fgets` reads through `Stdin`'s shared buffer so the leftover bytes
  survive to the next call.
- **`fgets` returning NULL.** Only when no byte at all was stored. With no input,
  both calls fail and `"fgets() failed."` is printed twice; with one line, only
  the second fails. A read error (e.g. stdin opened on a directory) is the same
  NULL case.
- **`data` stays `0.0F` when `fgets` fails**, which is why the failure path in
  `bad()` still ends in the divide by zero.
- **`atof` never reports errors.** It is `strtod(s, NULL)` with the end pointer
  thrown away, so trailing junk is ignored (`"5abc"` is 5.0) and an input with no
  valid subject sequence is `0.0` — including `""`, `"abc"`, `"-"`, `"."`, `"e5"`
  and a line that begins with a NUL byte. A leading sign is discarded in the
  no-conversion case, so `"-"` yields `+0.0`.
- **`atof` reads a C string.** The `fgets` buffer is interpreted only up to the
  first NUL, so `"3\0" "4"` is 3.0.
- **Narrowing to `float`.** `data` is `float` but the division is done in
  `double` (`100.0`). The `strtod` result is rounded to `f64` and then to `f32`
  before use; `atof(...) as f32` reproduces the same double rounding.
- **Byte-oriented input.** Neither `fgets` nor `atof` requires valid UTF-8, so
  the translation keeps input as `Vec<u8>` and never converts through `String`.
- **`printLine(NULL)` is unreachable.** No caller passes NULL, but the guard is
  preserved as `Option<&str>`.
- **`argc`/`argv` are unused**, so arguments change nothing.
- **Output format.** `printf("%s\n", ...)` and `printf("%d\n", ...)`: no padding,
  no precision, one trailing newline each. C's stdout is fully buffered when
  redirected and Rust's is line buffered, but since nothing is written to stderr
  the captured bytes are identical either way.

## Input classes covered by `tests/differential.rs`

Each branch in the C is an input class; the tests below name the branch they
reach. All of them assert stdout, stderr and exit status.

| Branch / behaviour in `main.c` | Test |
| --- | --- |
| both `fgets` return NULL (empty stdin) | `empty_input_both_fgets_fail` |
| first `fgets` succeeds, second returns NULL | `single_line_second_fgets_fails` |
| both `fgets` succeed; surplus lines unread | `two_or_more_lines_both_fgets_succeed` |
| 19-byte buffer limit; line split across calls | `line_longer_than_buffer_is_split_across_calls` |
| `argc`/`argv` ignored | `argv_is_ignored` |
| `fabs(data) > 0.000001` true | `b2g_guard_above_threshold` |
| `fabs(data) > 0.000001` false, incl. `0`, `-0`, NaN, exactly `1e-6` | `b2g_guard_at_or_below_threshold` |
| `bad()` divides by zero / converts out of range | `bad_divide_by_zero_and_out_of_range_conversion` |
| `bad()` in-range conversion, truncation toward zero, 2^31 boundary | `bad_in_range_and_boundary_conversions` |
| `strtod` signs, whitespace, decimal and exponent forms | `atof_accepts_signs_whitespace_and_forms` |
| `strtod` "no conversion performed" | `atof_rejects_incomplete_subject_sequences` |
| `strtod` `inf`/`infinity`/`nan`/`nan(chars)` | `atof_infinity_and_nan_spellings` |
| `strtod` hex significand and `p` exponent, incl. overflow/subnormal | `atof_hexadecimal_forms` |
| `double` -> `float` narrowing at rounding boundaries | `atof_float_rounding_boundaries` |
| NUL bytes, non-UTF-8 and control bytes | `binary_and_non_utf8_input` |
| every byte value 0x00..0xFF as a line | `each_single_byte_line` |
| stdout write failure ignored by `printf` | `unwritable_stdout_does_not_change_exit_status` |
| stdin read error takes the NULL branch | `unreadable_stdin_takes_the_fgets_failure_branch` |

## Additional checking not kept in the suite

Beyond the cases above, roughly 10,000 randomly generated inputs were compared
directly (random decimal, hexadecimal, `inf`/`nan` and raw-byte lines, plus
values chosen to sit next to the `1e-6` guard and the 2^31 conversion
boundary). After the fix, no input produced a difference in stdout, stderr or
exit status.

The suite was mutation-checked to confirm it is not vacuous: changing
`f64_to_int`'s out-of-range result from `i32::MIN` to `0` fails 9 tests, and
restoring `println!` in `print_line` fails
`unwritable_stdout_does_not_change_exit_status`.

## Status

- Both programs build without errors.
- `cargo test` passes: 18 tests, 0 failed, 0 ignored. No test is disabled,
  skipped or `#[ignore]`d.
- `c_src/` is unmodified.
