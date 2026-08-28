# Differential verification notes

Reference: `c_src/src/main.c` (built with CMake to `c_src/build/driver`).
Under test: `translation/src/main.rs` (built to `translation/target/*/driver`).

Both programs are compared by execution only: identical `argv`, then a byte-for-byte
diff of stdout, stderr, and the exit status (including death-by-signal).
See `translation/tests/differential.rs`.

## How the C branches — the enumerated input classes

| # | Condition in `main.c` | Input class | Observable result |
|---|---|---|---|
| 1 | `argc > 4 \|\| argc == 1` | no args; 4+ trailing args | two-line usage message, exit 1 |
| 2 | `argc >= 3`, `end == argv[2]` | non-numeric second arg | `Second argument must be an integer!` (**no newline**), exit 1 |
| 3 | `argc >= 3`, `start > len` | start past end, or any negative start | `Error: start is off the end of the string!`, exit 1 |
| 4 | `argc == 2` | string only | whole string, exit 0 |
| 5 | `argc == 4`, `end == argv[3]` | — | **unreachable**, see mismatch 2 below |
| 6 | `argc == 4`, `stop > len` | stop past end, or any negative stop | `Error: stop is off the end of the string!`, exit 1 |
| 7 | `argc == 4`, `stop <= start` | stop equal to or before start | `Error: stop must come after start!`, exit 1 |
| 8 | fallthrough | valid `[start, stop)` | `printf("%.*s\n", stop - start, argv[1] + start)`, exit 0 |

Boundary and representation classes layered on top of those: empty `argv[1]`;
`start == len` exactly; `start == len + 1`; `strtol` leading whitespace, `+`/`-`
sign, partial numeric prefixes, and non-ASCII digits; `long` → `int` truncation;
`strtol` `ERANGE` saturation to `LONG_MAX`/`LONG_MIN`; non-UTF-8 argument bytes;
and strings longer than stdio's internal buffer.

## Mismatches found

### 1. SIGPIPE disposition — real mismatch, fixed in the Rust

**Symptom.** With stdout attached to a pipe/socket whose reader is gone, and an
`argv[1]` large enough to force a `write()`:

| | exit status |
|---|---|
| C | killed by `SIGPIPE` (signal 13) |
| Rust (before) | exit code 0 |

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` during
runtime startup, before `main` is entered. Writes to a dead pipe therefore return
`EPIPE` instead of raising the signal, and the original translation discarded that
`io::Error` (`let _ = out.write_all(..)`), so the process ran to a clean exit. The C
program never touches the signal, keeps the default disposition, and dies inside
its write — either in `printf` or in the `exit`-time flush.

**Fix.** `restore_default_sigpipe()` in `src/main.rs` calls `signal(SIGPIPE, SIG_DFL)`
as the first statement of `main`, restoring the C disposition. Declared as a bare
`extern "C"` so no new crate dependency is introduced.

**Regression test.** `dying_on_a_closed_stdout_matches` — builds a `UnixStream`
socketpair, hands one end to the child as stdout, drops the other, and compares
the resulting exit status of both binaries.

### 2. `end == argv[3]` is dead code — confirmed, not a mismatch

The third-argument guard reads:

```c
stop = strtol(argv[3], NULL, 10);
if (end == argv[3]) { printf("Third argument must be an integer!"); return 1; }
```

`NULL` is passed as `endptr`, so `end` is *not* updated here; it still holds
whatever the `argv[2]` call left behind. `end` therefore points somewhere in
`[argv[2], argv[2] + strlen(argv[2])]`, while `argv[3] == argv[2] + strlen(argv[2]) + 1`
in the contiguous argv block Linux builds. The two can never be equal, so the
branch is unreachable and a non-numeric third argument silently yields `stop == 0`,
which then trips the `stop <= start` check instead.

The Rust encodes this as a literal `false` with a comment. Verified by
`non_numeric_stop_silently_becomes_zero`: `driver hello 0 abc` prints
`Error: stop must come after start!` in both programs, never the
"Third argument" message.

### 3. Behaviours that look like bugs and were deliberately preserved

None of these produced a diff; they are recorded because a naive translation
would get them wrong.

- **`start > len` / `stop > len` are unsigned comparisons.** `len` is `size_t`, so
  the `int` operand is converted to `unsigned long`. Every negative `start` or
  `stop` becomes a huge value and is rejected as "off the end" rather than being
  treated as an offset from the end. Rust: `(start as i64 as u64) > len`.
- **`stop <= start` is a signed comparison**, because both operands are `int`.
  The two checks in the same block therefore use different signedness.
- **`long` → `int` truncation.** `strtol` returns `long`, assigned to `int`.
  `driver hello 4294967298` prints `llo` (2^32 + 2 truncates to 2), and
  `driver hello 0 -4294967291` prints `hello` (truncates to 5).
- **`ERANGE` saturation then truncation.** `9223372036854775808` saturates to
  `LONG_MAX`, which truncates to `-1`, which is then rejected as off the end.
  `-9223372036854775809` saturates to `LONG_MIN`, truncating to `0`.
- **The "Second argument" message has no trailing newline**, unlike every other
  error message in the file.
- **All output, including errors, goes to stdout.** Neither program writes a
  single byte to stderr.
- **Check order.** The `argc` gate precedes all parsing; the second-argument
  checks precede all third-argument work; `stop > len` precedes `stop <= start`.
  So `driver hello 3 -1` reports "off the end", not "must come after start".
- **`start == len` is legal** (`>` not `>=`), yielding an empty substring and a
  bare newline.
- **`%.*s` and slicing are byte-oriented**, so multibyte characters are cut in
  half: `driver héllo 1 2` emits the single byte `0xC3`.
- **Arguments are arbitrary byte strings.** The Rust uses `args_os()` with
  `OsStrExt::as_bytes` and writes raw bytes, so invalid UTF-8 passes through
  unchanged instead of being lossily replaced.
- **`isspace` is evaluated in the C locale.** The program never calls
  `setlocale`, so only `' '`, `\t`, `\n`, `\v`, `\f`, `\r` are skipped by
  `strtol`, and only ASCII `0`-`9` count as digits — `U+0663` is not a digit.

## Cases deliberately left alone

`argc == 0` (reachable only via `execve` with an empty `argv`) would make the C
call `strlen(NULL)`. On this kernel an empty `argv` is normalised so that
`argc == 1`, and both binaries then print the usage error and exit 1 — verified
identical. There is no `argv` that reaches the C's null dereference, so the Rust
is not made to imitate a crash.

## Verification performed

- `cmake --build .` in `c_src/build` — clean.
- `cargo build --release` — clean, no warnings.
- `cargo test` and `cargo test --release` — 34 tests, 0 failed, 0 ignored.
- An additional out-of-tree randomised sweep of 15,143 `argv` vectors
  (`argc` 0 through 6, drawn from the numeric edge cases plus random byte
  strings) reported 0 mismatches on all three channels.
- `c_src/` sources unmodified; only the ignored CMake `build/` output tree was
  added.
