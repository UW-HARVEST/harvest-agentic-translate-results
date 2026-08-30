# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

Program under test: count up from an integer argument, printing each value,
stopping when the value ends in `9` (base 10).

Comparison method: both programs are built and then executed as subprocesses
with identical `argv`; stdout, stderr and exit status are compared byte for
byte (`translation/tests/differential.rs`). The Rust code is never loaded as a
library.

- C binary: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver`
- Rust binary: `cd translation && cargo build --release` → `translation/target/release/driver`

## Result

No behavioural mismatches remained to be fixed. `cargo build --release`
compiled without errors on the first attempt, and all 45 differential tests
pass. `c_src/` was not modified.

The sections below record the C behaviours that are easy to get wrong in a
translation, the input that exposes each one, and how the existing Rust
handles it. Each was confirmed by running both binaries, not by reading alone.

## Behaviours checked, with the input that exposes them

### 1. `int val = strtol(argv[1], &end, 10)` truncates `long` → `int`

`strtol` returns a 64-bit `long`, but it is assigned to an `int`. The C
silently keeps the low 32 bits.

| argv[1] | `strtol` result | `int val` | first line |
|---|---|---|---|
| `4294967296` (2^32) | 4294967296 | `0` | `0` |
| `4294967305` (2^32+9) | 4294967305 | `9` | `9` (stops at once) |
| `4294967286` (2^32-10) | 4294967286 | `-10` | `-10` |
| `9223372036854775807` (`LONG_MAX`) | `LONG_MAX` | `-1` | `-1` |
| `-9223372036854775808` (`LONG_MIN`) | `LONG_MIN` | `0` | `0` |

A translation that parsed into `i64` and printed that, or that used `i32`
parsing and rejected out-of-range input as an error, would diverge here. The
Rust parses into `i64` and then does `parsed as i32`, matching the truncation.
Tests: `truncation_*`, `long_max_truncates_to_minus_one`,
`long_min_truncates_to_zero`.

### 2. `strtol` range overflow clamps, it does not fail the program

For input that exceeds `long`, glibc sets `errno = ERANGE` and returns
`LONG_MAX`/`LONG_MIN`. The C never checks `errno`, so those clamped values are
truncated as above and the program exits 0.

- `9999999999999999999999` → `LONG_MAX` → `-1` → prints `-1 … 9`, exit 0
- `-9999999999999999999999` → `LONG_MIN` → `0` → prints `0 … 9`, exit 0

Rust mirrors this by saturating to `i64::MIN`/`i64::MAX` on overflow rather
than reporting an error. Test: `long_max_truncates_to_minus_one`,
`long_min_truncates_to_zero`.

### 3. `val % 10 == 9` is never true for negative `val`

C's `%` truncates toward zero, so the remainder takes the sign of the left
operand: `-9 % 10 == -9`, not `9`. Every negative start therefore counts all
the way up through zero to `9`.

- `-9` prints `-9, -8, …, 9` (19 lines), it does **not** stop immediately
- `-3` prints `-3 … 9`
- `-2147483648 % 10 == -8`, so `INT_MIN` counts up ~2.1 billion lines

Rust's `%` on `i32` has the same truncating semantics, so this needed no
special handling — but a translation using a Euclidean/`rem_euclid` remainder
would stop early on `-9` and lose 18 lines. Tests:
`negative_nine_does_not_stop_early`, `negative_*`.

### 4. Signed overflow past `INT_MAX` wraps

`2147483639` is the largest `int` satisfying `val % 10 == 9`. Starting at
`2147483647` the loop prints `INT_MAX`, then `val++` overflows — undefined
behaviour in C, but the compiled binary wraps to `INT_MIN` and keeps counting:

```
2147483647
-2147483648
-2147483647
...
```

Rust uses `wrapping_add(1)` to reproduce this; plain `+ 1` would panic in debug
and the two programs would disagree on both output and exit status. Because
these inputs emit gigabytes, the tests compare a bounded 4 KiB prefix of stdout
(`assert_same_prefix`) instead of running to completion. Tests:
`int_max_wraps_to_int_min`, `overflow_crosses_int_max_boundary`,
`int_min_start_runs_for_billions_of_iterations`,
`overflow_prefix_contains_the_wrap`.

### 5. `end == argv[1]` is the *only* validation

The C checks solely whether `strtol` converted zero characters. Anything with a
leading (post-whitespace, post-sign) digit is accepted and trailing garbage is
ignored.

Accepted, exit 0:

| argv[1] | starts at | why |
|---|---|---|
| `+7` | 7 | sign is consumed by `strtol` |
| `007` | 7 | base 10, not octal |
| `  42abc` | 42 | leading whitespace skipped, trailing garbage ignored |
| `0x1F` | 0 | base 10 stops at `x` |
| `3.7` | 3 | stops at `.` |
| `1e3` | 1 | stops at `e` |
| `5 6` | 5 | stops at the space |

Rejected with `Error: first argument must be an integer!` and exit 1:
`""`, `abc`, whitespace-only (`"   "`, `"\t"`, `"\n"`, `"\r"`, `"\x0b"`,
`"\x0c"`), sign-only (`+`, `-`, `--5`, `+ 5`), non-digit leaders (`.5`, `e5`,
`x10`, `#3`), and non-ASCII digit lookalikes (`٣`, `２`).

Note the whitespace-only case specifically: `strtol` consumes the whitespace
but, having converted nothing, resets `*end` back to `nptr`, so the check
fires. A translation that reported `end` as the post-whitespace offset would
wrongly accept `"   "`. The Rust returns offset `0` whenever no digits were
consumed. Tests: `parse_error_*`, plus the accepting tests in the happy-path
section.

### 6. Both error messages go to **stdout**, not stderr, and return 1

```c
printf("Error: should only be a single (integer) argument!\n");
printf("Error: first argument must be an integer!\n");
```

`printf`, not `fprintf(stderr, ...)`. Every test asserts `stderr` is empty, so
routing either message to stderr would fail even though the visible terminal
output looked identical.

Exact bytes, including the trailing newline and the `!`:

- `Error: should only be a single (integer) argument!\n` (argc != 2)
- `Error: first argument must be an integer!\n` (nothing parsed)

### 7. Order of the two checks

`argc != 2` is tested *before* `strtol` runs. So `driver abc def` reports the
argc error, not the parse error — the invalid `abc` is never examined. Tests:
`argc_two_extra_args`, `argc_three_extra_args`, `argc_two_args_second_empty`.

### 8. `argv` is bytes, not UTF-8

C does no validation of argument encoding. A translation using
`std::env::args()` would **panic** on a non-UTF-8 argument, producing a Rust
panic message on stderr and exit code 101 where the C prints its parse error
and exits 1.

- `\xff\xfe` → `Error: first argument must be an integer!`, exit 1
- `12\xff` → prints `12 … 19`, exit 0

Rust uses `args_os()` with `OsStrExt::as_bytes()`. Tests:
`parse_error_invalid_utf8_arg`, `digits_followed_by_invalid_utf8`.

### 9. SIGPIPE disposition

The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`; a C program leaves
it at `SIG_DFL`. For the unbounded inputs this is observable:

```
driver 2147483647 | head -1   # C: killed by signal 13, shell reports 141
```

Without intervention the Rust program would instead get a write error, ignore
it (as the C ignores `printf` failures), and exit 0. `reset_sigpipe()` restores
`SIG_DFL`, so both are killed by signal 13. Test:
`closed_stdout_kills_both_with_sigpipe`.

### 10. Loop shape: print-then-test

The loop is `while(1) { print; if (val%10==9) break; val++; }`. The value is
always printed at least once, and the terminating value *is* printed. So `9`
produces exactly `9\n` (one line, not zero), and `12` produces `12`–`19`
inclusive. Tests: `single_item_already_ends_in_nine`,
`counts_from_nineteen_stops_immediately`, `every_last_digit_zero_through_nine`.

## Additional fuzzing

Beyond the enumerated cases, both binaries were run against 1372 generated
arguments — 1200 random strings over an alphabet of digits, signs, whitespace
(including `\x0b`/`\x0c`), radix and float punctuation, letters, invalid UTF-8
bytes and non-ASCII digits, plus structured values swept around `0`, `±9`,
`±10`, `INT_MAX`, `INT_MIN`, `2^32`, `LONG_MAX`, `LONG_MIN` and `2^64`, and
long runs of `9`s and leading `0`s. Output was compared as a bounded 8 KiB
prefix so the overflow cases terminated. **0 mismatches.**

## Proving the suite has teeth (mutation check)

A suite that passes tells you nothing unless it can also fail. Seven deliberate
bugs were injected into `src/main.rs` one at a time; each was caught, and the
original file was restored afterwards.

| Injected bug | Result |
|---|---|
| `parsed as i32` → saturate to `i32::MIN/MAX` | 5 failed (`long_max_truncates_to_minus_one`, `long_min_truncates_to_zero`, `truncation_*`) |
| `val % 10` → `val.rem_euclid(10)` | 10 failed (all negative-start tests) |
| parse error via `eprint!` (stderr) instead of `printf` (stdout) | 7 failed |
| argc message loses its trailing `\n` | 5 failed (all `argc_*`) |
| `reset_sigpipe()` removed | 1 failed (`closed_stdout_kills_both_with_sigpipe`) |
| `args_os()` → `args()` (panics on non-UTF-8) | 2 failed (`parse_error_invalid_utf8_arg`, `digits_followed_by_invalid_utf8`) |
| `strtol` end offset reported after whitespace instead of `0` | 7 failed (all `parse_error_*`) |

One mutation was found to be **behaviourally equivalent** rather than a missed
bug: replacing the unix `as_bytes()` path with `to_string_lossy()`. Invalid
bytes become U+FFFD, which is neither a digit, a sign nor whitespace, and all
characters `strtol` actually inspects are ASCII and pass through unchanged — so
the parsed prefix is identical for every input. It is kept as `as_bytes()`
anyway, since that is what C does.

### A note on test robustness

Two of these mutants originally made the suite **hang** instead of fail: a
wrong `int` conversion turns a 11-line run into a ~2.1-billion-line one, and an
ignored `SIGPIPE` makes the program spin forever on a broken pipe. Both were
detected, but only by wedging the run. `run()` and the SIGPIPE helper therefore
enforce a 20 s / 10 s deadline and a 1 MiB stdout cap, and panic with a
diagnostic when exceeded. The saturate mutant now fails in 0.16 s. The
genuinely unbounded C inputs are handled separately by `assert_same_prefix`,
which compares a bounded 4 KiB prefix.

## Completion gate

- [x] both programs build with no errors
- [x] every enumerated input produces identical stdout, stderr and exit status
- [x] `cargo test` passes in `translation/` (45 tests)
- [x] no test is disabled, skipped or `#[ignore]`d
- [x] nothing in `c_src/` has been modified
