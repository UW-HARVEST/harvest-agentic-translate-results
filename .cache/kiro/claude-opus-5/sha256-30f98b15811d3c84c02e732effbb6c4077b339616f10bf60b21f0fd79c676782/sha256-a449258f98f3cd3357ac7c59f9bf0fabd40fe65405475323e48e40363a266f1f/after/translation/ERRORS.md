# ERRORS.md — mismatches found while verifying the C → Rust translation

Ground truth: `c_src/src/main.c`, built with `cmake` (no optimisation flags, gcc
on glibc/Linux x86-64). Comparison method: run both binaries as subprocesses on
the same stdin, diff stdout, stderr and exit status byte for byte
(`translation/tests/differential.rs`).

## How the C program branches

```
main()
  char in[100] = "";                  <- all 100 bytes zeroed
  fgets(in, 100, stdin)               <- at most 99 bytes, stops after '\n'
  parse_val(in, &x) ?  run(); run()  :  printf("An error occurred\n")
  return 0                            <- always 0

parse_val(str, val)
  errno = 0
  tmp = strtol(str, &endp, 10)
  accept iff  endp != str  &&  errno == 0  &&  INT_MIN <= tmp <= INT_MAX
```

Notable: `parse_val` never checks `*endp`, so `"42abc"` is accepted. `main`
returns 0 on both paths, and every `printf` return value is discarded.

## Mismatches found and fixed

### 1. Exit status and stderr on a failing stdout (ENOSPC)

Reproducer: `echo 42 | ./driver > /dev/full`

| | stdout | stderr | status |
|---|---|---|---|
| C | (nothing written) | empty | `0` |
| Rust (before) | (nothing written) | `thread 'main' panicked ... failed printing to stdout: No space left on device (os error 28)` | `134` (SIGABRT) |

Cause: the translation used the `print!` macro. `print!` **panics** when the
underlying write fails. The C code calls `printf` and throws the return value
away, so a failed write is invisible and `main` still returns 0. With
`panic = "abort"` in the release profile the panic became SIGABRT, so both the
exit status and stderr diverged.

Fix: `src/main.rs` now routes all output through a small `COut` struct that
accumulates bytes in a `Vec<u8>` and performs a single `write_all` + `flush` at
exit with both `Result`s discarded (`let _ = ...`). Buffering everything until
exit also matches glibc's behaviour here: `stdout` is fully buffered when it is
not a terminal and this program emits at most 8 short lines, well under
`BUFSIZ`, so the real C program never flushes before `exit` either.

Covered by `stdout_write_error_is_ignored_like_c`.

### 2. SIGPIPE disposition — writing to a closed pipe

Reproducer: `echo 42 | ./driver | true` (reader exits before the writer writes)

| | status |
|---|---|
| C | killed by signal 13 (shell reports `141`) |
| Rust (before) | `0` |

Cause: the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs. A C
program inherits the default disposition, so its write to the closed pipe kills
it. The Rust binary instead got `EPIPE`, and once fix #1 made write errors
silent it exited 0 — a silent divergence in exit status with identical (empty)
stdout.

Fix: `reset_sigpipe()` at the top of `main` restores `SIG_DFL` for `SIGPIPE`
via a direct `extern "C" { fn signal(...) }` declaration (no new dependency).

Covered by `sigpipe_disposition_matches_c`.

Note that fixes #1 and #2 are independent: reverting either one alone makes
exactly one of the two tests fail. Both were verified that way.

## Behaviours deliberately replicated, not "fixed"

These looked like bugs in the C but are reproduced exactly.

* **Trailing garbage is accepted.** `parse_val` ignores `*endp`, so `"42abc"`,
  `"2.5"`, `"1e5"`, `"1 2"`, `"0x10"` and `"1_000"` all parse as `42`, `2`, `1`,
  `1`, `0` and `1` respectively and take the success path. The Rust must not
  require the whole string to be consumed. (`accepts_trailing_garbage_after_digits`)
* **Signed `int` overflow wraps.** `add_bedrooms` does `bedrooms += extra` and
  `run` is called twice, so `x = 2147483647` computes `5 + x` and then `5 + 2x`,
  both of which overflow. This is UB in C, but the built binary wraps two's
  complement. The Rust uses `wrapping_add`; `saturating_add` or a plain `+`
  (which panics in debug) would both diverge. (`bedrooms_overflow_wraps_like_c`)
* **`fgets`, not `scanf`.** Only the first line is consumed and at most 99
  bytes. `"42\n99\n"` uses `42`. A 100-digit line is truncated to 99 digits,
  which then overflows and takes the *error* path; truncating a long line can
  also change the parsed value (`"0"*98 + "5" + "9"*20` parses as `5`).
  (`fgets_reads_only_the_first_line`, `fgets_truncates_at_99_bytes`)
* **Embedded NUL ends the string.** The buffer is NUL-terminated, so `"4\0002"`
  parses as `4`, and a leading NUL yields the empty string and the error path.
  (`embedded_nul_terminates_the_c_string`, `rejects_leading_nul`)
* **`errno == 0` vs. the range check are different rejections.**
  `"9223372036854775807"` is rejected by the `INT_MAX` comparison with
  `errno == 0`; `"9223372036854775808"` is rejected because `strtol` set
  `ERANGE`. Both print the same message, but the Rust `strtol` emulation has to
  model the `ERANGE` flag and the saturating return value separately from the
  range test. `strtol` must saturate to `LONG_MAX`/`LONG_MIN`, and
  `"-9223372036854775808"` must *not* set `ERANGE`.
  (`rejects_out_of_int_range_but_in_long_range`, `rejects_erange_overflow`)
* **`strtol` whitespace set.** Space, `\t`, `\v`, `\f`, `\r`, `\n` are all
  skipped before the optional sign; a space *between* the sign and the digits
  (`"- 5"`) is not. (`accepts_leading_whitespace_forms`, `rejects_non_numeric`)
* **`exit 0` on every input path**, including a failed `fgets` (closed stdin,
  stdin is a directory). (`closed_stdin_matches_c`, `stdin_from_directory_matches_c`)
* **`%.1f`** — `bathrooms` only ever takes the values 2.5, 3.5 and 4.5, all
  exactly representable, so no rounding-mode difference between glibc and Rust
  can surface. `{:.2}` instead of `{:.1}` was checked to fail 18 tests, so the
  precision is pinned by the suite.

## No mismatch found in

8000 additional pseudorandom inputs (random byte strings, random decimal
integers, strings near the 99-byte `fgets` boundary, and full 0–255 byte
sweeps) produce identical stdout, stderr and exit status. A boundary sweep over
`2^n - 1`, `2^n`, `2^n + 1` and their negations for `n = 0..70` is part of the
committed suite (`sweeps_powers_of_two_and_boundaries`).
