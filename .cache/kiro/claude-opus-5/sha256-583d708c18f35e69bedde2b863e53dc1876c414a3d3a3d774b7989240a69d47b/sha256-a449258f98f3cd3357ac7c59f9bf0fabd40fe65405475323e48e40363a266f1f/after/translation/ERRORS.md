# Differential verification: mismatches found and fixed

The C in `c_src/src/main.c` is the ground truth. This file records every
behavioral difference found between it and the Rust translation, and what caused
each one. Verification is by running both binaries as subprocesses on identical
stdin and comparing stdout, stderr and exit status byte for byte
(`translation/tests/differential.rs`).

## Program under test

```c
void driver(int x) {
    register int y = 2*x;
    y += 300;
    printf("%d\n", y);
}

int main() {
    int x = 0;
    scanf("%d", &x);
    driver(x);
    return 0;
}
```

The only observable output is one line, `2*x + 300`. `scanf`'s return value is
discarded, so a failed conversion leaves `x` at its initializer `0` and the
program prints `300`. `main` always returns 0.

## Mismatch 1 — exit status when stdout has no reader (SIGPIPE)

**Status: found and fixed.**

Input: any (`5\n`), with stdout connected to a pipe whose read end is already
closed.

| | stdout | stderr | exit status |
|---|---|---|---|
| C | empty | empty | killed by signal 13 (`SIGPIPE`), wait status 141 |
| Rust (before fix) | empty | empty | exit 0 |

**Cause.** Rust's standard runtime sets `SIGPIPE` to `SIG_IGN` before `main`
runs. The failing `write` therefore returns `EPIPE`, which the translation
discards with `let _ = ...`, and the process exits 0. The C program inherits the
default `SIGPIPE` disposition and is killed by the signal instead.

This is a divergence in the Rust *runtime*, not in the translated logic, so no
amount of care inside `driver` would have caught it — it is only visible when the
exit status is compared, which is exactly the failure mode the task warns about
("a test that checks only stdout will pass while the Rust program exits 0 where
the C exits 1").

**Fix.** `restore_default_sigpipe()` in `src/main.rs` calls
`signal(SIGPIPE, SIG_DFL)` as the first statement of `main`, undoing the
runtime's change. Regression test: `sigpipe_on_closed_stdout_matches`.

## Verified-equivalent behaviors (no mismatch, but non-obvious)

These were each checked against the C binary and already matched. They are
listed because they are the places a translation is most likely to drift, and
each has a test that would catch a regression.

- **`scanf` skips across newlines.** Whitespace skipping before `%d` consumes
  `\n`, so `"\n\n\n8"` still yields `x = 8`. A `read_line`-based translation
  would have failed here. The translation models this with a byte-level
  `getc`/`ungetc` loop over the C-locale `isspace` set
  (space, `\t`, `\n`, `\v`, `\f`, `\r`). Test: `scanf_reads_across_newlines`.

- **EOF versus matching failure are different branches with the same output.**
  Empty input makes `scanf` return `EOF` (−1); `"abc"` makes it return 0. Both
  leave `x = 0` and print `300`, so they are indistinguishable from the outside
  — but only because the C discards the return value. Tests:
  `empty_and_whitespace_only_input`, `matching_failure_leaves_x_at_zero`.

- **Signed overflow in `2*x + 300` wraps.** `x = INT_MAX` prints `298`,
  `x = 1073741824` prints `-2147483348`. This is undefined behavior in C, but
  the binary as built wraps, so the translation uses `wrapping_mul` /
  `wrapping_add`. Plain `*` and `+` would panic under `cargo test`'s debug
  overflow checks and would still differ from the C in release. Test:
  `arithmetic_overflow_in_driver`.

- **Out-of-range input saturates at `long`, then narrows to `int`.** glibc's
  `%d` conversion clamps to `LONG_MAX`/`LONG_MIN` and stores the `long` into the
  `int` argument. So `"99999999999999999999"` gives `x = (int)LONG_MAX = -1`
  and prints `298`, while `"-99999999999999999999"` gives
  `x = (int)LONG_MIN = 0` and prints `300`. The asymmetry is easy to get wrong.
  Tests: `values_exceeding_int_range`, `values_exceeding_long_range`,
  `very_long_digit_runs` (digit runs up to 4097 characters).

- **Values between `int` and `long` range are truncated, not clamped.**
  `"4294967296"` (2^32) gives `x = 0` → `300`; `"4294967297"` gives `x = 1` →
  `302`. Test: `values_exceeding_int_range`.

- **`%d` stops at the first non-digit and leaves the rest unread.** `"12abc"`
  → `324`, `"0x10"` → `300` (the `0` converts, `x` stops it), `"1 2"` → `302`.
  Test: `number_followed_by_trailing_junk`.

- **A sign with no digits is a matching failure.** `"-"`, `"+"`, `"- 5"`,
  `"--5"` all leave `x = 0` → `300`. Test:
  `matching_failure_leaves_x_at_zero`.

- **Embedded NUL and high bytes are ordinary non-digits.** They terminate or
  prevent the conversion; neither program treats them as end of input. Test:
  `non_ascii_and_control_bytes`.

- **Output formatting.** `printf("%d\n", y)` emits the digits, a minus sign when
  negative, and exactly one trailing newline, with nothing on stderr. Compared
  byte for byte across every value in −400..=400 plus a 300-value pseudo-random
  64-bit sweep. Tests: `dense_sweep_of_small_values`,
  `pseudo_random_wide_value_sweep`.

- **Writes that fail without SIGPIPE are ignored identically.** With stdout on
  `/dev/full` or with fd 1 closed, both programs print nothing and exit 0; C
  ignores the `printf`/exit-flush failure and so does the translation.

- **Extra `argv` entries are ignored.** `main` takes no parameters in either
  program. Test: `extra_arguments_are_ignored`.

## Result

Both programs build without errors, and every enumerated input produces
identical stdout, stderr and exit status. `cargo test` passes with 18 tests, none
disabled, skipped or `#[ignore]`d. Nothing in `c_src/` was modified.
