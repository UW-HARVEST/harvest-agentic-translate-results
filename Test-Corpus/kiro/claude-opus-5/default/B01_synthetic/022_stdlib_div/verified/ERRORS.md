# ERRORS.md — differential verification log

Scope: `c_src/src/main.c` (the ground truth) vs `translation/src/main.rs`.

```c
int main() {
    int x = 1, y = 1;
    scanf("%d %d", &x, &y);
    div_t result = div(x, y);
    printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
    return 0;
}
```

## Result

**No mismatches were found.** Every input class enumerated below produced
byte-identical stdout, byte-identical stderr and an identical exit status
(including identical terminating signal) between the C binary and the Rust
binary. No change to `translation/src/main.rs` was required, and nothing in
`c_src/` was modified.

Evidence:

- `cargo test` in `translation/`: 16 tests, ~230 distinct inputs, all passing.
- An additional randomized sweep of 4500 inputs (random byte soup over digits,
  signs, all six whitespace characters, `NUL`, `0xff`, `0x80`, letters and
  punctuation; plus structured numeric pairs drawn from the `int`, `long` and
  10^25 ranges) against the `--release` binaries: 0 mismatches.

## Input classes enumerated from the C source

The C program has no `if` statements, so the branch points are all inside the
library calls it makes. Each one below is a distinct behavior the translation
had to reproduce, and each has a test.

| Branch point | Input class | Observed C behavior | Test |
|---|---|---|---|
| `scanf` input failure on the 1st `%d` | empty / whitespace-only stdin | `x` and `y` keep their initializer `1` → `quotient: 1, remainder: 0`, exit 0 | `empty_and_whitespace_only_input` |
| `scanf` input failure on the 2nd `%d` | a single item (`5`, `5\n`, `5 `) | `y` keeps `1` | `single_item_input` |
| `scanf` matching failure | `abc`, `0x10`, `1e3`, `.5`, `-`, `+`, `- 5`, `+-5`, `,;:` | affected variable keeps `1`; return value is never checked | `matching_failure_leaves_initializers` |
| `%d` whitespace skip | newlines/tabs/`\v`/`\f`/`\r` between and before items | `scanf` reads across line boundaries (unlike `fgets`) | `scanf_reads_across_newlines` |
| `%d` sign handling | `+5 +3`, `5-3`, `-7 -2` | optional sign accepted; `5-3` parses as `5` and `-3` | `two_item_happy_paths` |
| `%d` → `strtol` overflow | `2147483648`, `4294967296`, `LONG_MAX`, `LONG_MAX+1`, `LONG_MIN-1`, `2^64+1`, 23 nines, 4096 nines | glibc converts via `strtol`, which **saturates** at `LONG_MAX`/`LONG_MIN`, then assigns to `int` (truncation). `99999999999999999999999` → `LONG_MAX` → `-1`; the negative form → `LONG_MIN` → `0` | `overflow_saturates_then_truncates`, `very_long_digit_runs` |
| `int` extremes | `INT_MAX`, `INT_MIN` as numerator and denominator | wrap-free truncating division; `INT_MIN / INT_MIN == 1` | `int_boundaries` |
| `div` with `denom == 0` | `0 0`, `5 0`, `-5 0`, `5 -0`, `5 0000` | x86 `idiv` traps: process killed by **SIGFPE (signal 8)**, empty stdout, empty stderr, no exit code | `division_faults_match` |
| `div` with `INT_MIN / -1` | `-2147483648 -1` | same divide fault: **SIGFPE**, empty output | `division_faults_match` |
| `div` sign of remainder | all pairs in `[-6, 6]²` with non-zero divisor | C truncates toward zero, so the remainder takes the numerator's sign | `small_operand_sweep` |
| `printf` format | `7 2` | exactly `quotient: 3, remainder: 1\n` — one trailing newline, one space after each colon and after the comma | `output_format_is_byte_exact` |
| `main()` takes no parameters | extra `argv` entries | ignored by both | `command_line_arguments_are_ignored` |
| raw byte handling | `NUL`, `0xff`, `0x80`, UTF-8 `é` | no special treatment; they are simply non-digit, non-space bytes that cause a matching failure or terminate a conversion | `non_ascii_and_nul_bytes` |

## Behaviors that look like bugs and were deliberately preserved

1. **`scanf`'s return value is discarded.** Bad or absent input therefore
   silently yields `1 / 1`. The translation reproduces this by writing through
   `&mut` only on a successful conversion.
2. **Undefined behavior on division.** `div(x, 0)` and `div(INT_MIN, -1)` are UB
   in C; on x86-64 they raise SIGFPE. Rust would instead panic (exit 101 with a
   message on stderr), which would not match, so the translation explicitly
   calls `raise(SIGFPE)` before performing the division for exactly those two
   operand shapes. Verified: both binaries report `returncode -8` with empty
   stdout and stderr.
3. **Saturating-then-truncating integer conversion.** `%d` does not wrap on
   overflow the way a naive `as i32` on a wrapping accumulator would. The
   translation accumulates in `i64`, saturates to `i64::MIN`/`i64::MAX`, and
   only then casts to `i32`, which is what glibc's `strtol`-based conversion
   does. This is glibc-specific and is the single most likely source of a
   silent mismatch; see the table row above for the exact witnesses.
4. **One byte of pushback.** A matching failure `ungetc`s the offending
   character. The translation models this with a one-byte peek buffer. It is not
   observable in this program's output (nothing reads stdin afterwards), but it
   keeps the parse of inputs like `5-3` and `- 5` faithful.

## Confirming the test suite has teeth

The harness was mutation-tested: three deliberate defects were injected into
`translation/src/main.rs`, the suite was run, and each was caught. The source
was restored to its original bytes afterwards (verified with `diff`).

| Injected defect | Caught by | Reported difference |
|---|---|---|
| `raise(SIGFPE)` → `raise(0)`, so the process aborts instead of faulting | `division_faults_match` | all 7 cases: `exit status differs: C Err(8) vs Rust Err(6)` |
| trailing `\n` removed from the `printf` format | `output_format_is_byte_exact` | stdout byte comparison failed |
| overflow wraps instead of saturating | `overflow_saturates_then_truncates` | 6 of 17 cases diverged |

This matters because a suite that only checks stdout would pass the first defect
and a suite that only checks the happy path would pass all three.
