# Differential testing: mismatches found and their causes

The C program in `c_src/` is ground truth. `translation/` must produce
byte-identical stdout, byte-identical stderr and an identical exit status for
every input. This file records each mismatch found while comparing the two
binaries as subprocesses, and what caused it.

## The program under test

```c
void driver(int x) {
    auto int y = 2*x;
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

There is no explicit control flow. The behavioral surface is entirely in three
places: what `scanf("%d", &x)` consumes and stores, whether it stores anything
at all (on failure `x` keeps its `0` initializer), and how `2*x + 300` behaves
in 32-bit `int`.

## Mismatch 1 — exit status on a broken stdout pipe

Status: **found and fixed.**

Input: any input, with stdout connected to a pipe whose read end is closed.

| | stdout | stderr | status |
|---|---|---|---|
| C | *(lost to the pipe)* | empty | killed by signal 13 (`SIGPIPE`) |
| Rust, before the fix | *(lost to the pipe)* | empty | exit 0 |

Cause: the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs. The C
program keeps the default disposition, so its `printf` write to a pipe with no
reader terminates it with signal 13. In Rust the same write returned `EPIPE`,
which `printf`'s translation discards exactly as C's ignored return value does
— so the write "failed silently" and the program went on to exit 0. Only the
termination status differed, and only stdout-and-exit-status comparison catches
it; comparing stdout alone shows nothing, since neither program's output
survives the broken pipe.

Fix: `restore_default_sigpipe()` in `src/main.rs` resets `SIGPIPE` to `SIG_DFL`
as the first statement of `main`, via a direct `signal(2)` declaration (the
crate has no dependencies). Covered by `broken_stdout_pipe_matches`.

## Mismatch 2 — a test-harness defect that hid mismatch 1

Status: **found and fixed** (in the test, not the translation).

The first version of `run_with_closed_stdout` built its pipe with `pipe(2)`.
Those file descriptors do not have close-on-exec set, so the spawned child
inherited the read end. The pipe therefore still had a reader, the write
succeeded, and **both** programs exited 0 — the test passed while measuring
nothing. It was caught by mutation testing: deleting the `SIGPIPE` fix from
`src/main.rs` did not make the suite fail.

Fix: the test now uses `pipe2(fds, O_CLOEXEC)`, so neither end leaks into the
child. `dup2` onto the child's fd 1 clears `CLOEXEC`, so the child's stdout
still works. With this change C reports signal 13, and a Rust build lacking the
`SIGPIPE` reset reports exit 0, so the test discriminates.

## Behaviors that look like bugs and were deliberately preserved

These were verified to already match and are **not** mismatches. They are
recorded because each is a plausible place for a translation to "fix" the C and
thereby diverge.

### `scanf` reads across newlines

`%d` skips any run of leading whitespace, newlines included, so `"\n\n\n5"`
yields 5. A translation built on line-oriented reading (`fgets`, `read_line`,
`lines()`) would return nothing for that input and print `300`. The translation
reads byte by byte and treats all six C whitespace characters (space, `\t`,
`\n`, `\v`, `\f`, `\r`) as skippable. Covered by
`scanf_reads_across_newlines`; the mutant that narrows the whitespace set to
space only is caught by 4 tests.

### Conversion failure leaves `x` at 0, and the program still exits 0

`scanf`'s return value is discarded. On input failure (EOF first) or matching
failure (first non-whitespace byte cannot begin an integer) nothing is stored,
`x` stays 0, and the program prints `300` and exits 0. There is no error
message and no non-zero exit status on any input. So `""`, `"abc"`, `"-"`,
`"."`, `"_5"`, `"\x005"` and whitespace-only input all print `300`.

`\0` is worth calling out: it is neither whitespace nor a digit, so `"\x005"`
is a matching failure and prints `300`, not `310`.

### `%d` stops at the first non-digit, mid-token

`"1.5"` reads 1, `"0x10"` reads 0 and stops at the `x`, `"5e3"` reads 5,
`"12abc"` reads 12. Only the first conversion is performed; the rest of stdin
is never read.

### Two independent overflow points

`2*x + 300` is computed in 32-bit `int` and wraps. `x = 1073741824` prints
`-2147483348`; `x = INT_MAX` prints `298`. Signed overflow is undefined in C,
but the compiled program wraps two's-complement, so the translation uses
`wrapping_mul`/`wrapping_add` to reproduce what the binary does.

Separately, `%d` converts the digit run with `strtol` into a `long` and stores
it through an `int *`. Values that exceed `int` are truncated to 32 bits
(`"4294967296"` stores 0 and prints `300`), and values that exceed `long`
saturate at `LONG_MAX`/`LONG_MIN` before that truncation. `LONG_MAX` truncates
to `-1`, so every sufficiently large positive input — `"9223372036854775808"`,
`"99999999999999999999"`, 100 000 nines — prints `298`. `LONG_MIN` truncates to
`0`, so the negative counterparts print `300`. The translation mirrors this
with saturating `i64` accumulation followed by a wrapping cast to `i32`;
replacing either the saturation or the truncating cast is caught (3 and 5 tests
respectively).

### Output format

Exactly `%d` and one `\n`: no field width, no padding, no thousands separator,
one line, always a trailing newline. Adding a space or dropping the newline is
caught by 21 tests.

## How the suite was validated

Ten mutations were introduced into `src/main.rs` one at a time and the suite was
re-run against the unmodified C binary:

| Mutation | Result |
|---|---|
| `+300` → `+301` | caught (20 tests) |
| `2*x` → `3*x` | caught (14 tests) |
| extra space in the format string | caught (21 tests) |
| trailing newline removed | caught (21 tests) |
| whitespace set narrowed to `' '` only | caught (4 tests) |
| saturating accumulation → wrapping | caught (3 tests) |
| matching-failure check removed | caught (8 tests) |
| truncating cast → clamp to `int` range | caught (5 tests) |
| `SIGPIPE` reset removed | caught (1 test, after mismatch 2 was fixed) |
| sign-then-EOF returns `Some(0)` instead of `None` | not caught — **equivalent** |

The last row is not a gap. `main` initializes `x` to 0, so storing 0 and
storing nothing are indistinguishable; no input can separate them.

Alongside the enumerated cases the suite runs 400 randomized inputs over the
byte alphabet `%d` branches on, 400 randomized signed digit strings of 1–25
digits, and every value in `-300..=300` plus the ±2 neighborhood of each
overflow boundary. A separate ad-hoc fuzz of 3 000 random inputs produced no
mismatches.

## Current state

22 tests, all passing, none `#[ignore]`d, skipped or disabled. Both the debug
binary Cargo hands the harness and the `--release` binary are compared against
C on every case. Nothing in `c_src/` was modified; only the generated
`c_src/build/` tree was created.
