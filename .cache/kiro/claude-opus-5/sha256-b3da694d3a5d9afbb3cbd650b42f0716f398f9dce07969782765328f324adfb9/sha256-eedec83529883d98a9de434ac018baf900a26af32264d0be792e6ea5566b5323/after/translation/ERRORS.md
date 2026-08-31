# Differential verification notes

C ground truth: `c_src/src/main.c`. Rust under test: `translation/src/main.rs`.

## How each program is run

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./c_src/build/driver            # reads stdin, writes stdout

# Rust
cd translation && cargo build --release
./translation/target/release/driver
```

The test suite (`translation/tests/differential.rs`) spawns both as
subprocesses, feeds identical stdin bytes, and compares stdout, stderr and exit
status. It builds the C binary with CMake on first use if `c_src/build/driver`
is absent, so `cargo test` is self-sufficient. Nothing is loaded as a library.

## What the program does

```c
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

`driver` `memcpy`s the `int` into `char raw[4]` and prints the four bytes with
`printf("%02x")`, then a newline. There is no other output, no stderr writes,
and the exit code is always 0 on a normal run. So the only thing that can
differ is the `int` value left in `x`, and the process exit status.

## Input classes enumerated from the C source

| Class | Example stdin | Resulting `x` | stdout |
|---|---|---|---|
| input failure (EOF before any non-space) | `""`, `"   "`, `"\n"` | untouched, `0` | `00000000` |
| matching failure (first non-space is not sign/digit) | `"abc"`, `".5"`, `"\0"` | untouched, `0` | `00000000` |
| sign then non-digit (matching failure) | `"-"`, `"- 5"`, `"+-5"` | untouched, `0` | `00000000` |
| plain value | `"42"` | `42` | `2a000000` |
| leading whitespace, crossing newlines | `"\n\n\t 7"` | `7` | `07000000` |
| trailing junk after digits | `"12abc"`, `"3.9"`, `"0x10"` | first run only | `0c000000` etc. |
| second item never read | `"3 4"` | `3` | `03000000` |
| `int` boundaries | `"2147483647"` / `"-2147483648"` | as written | `ffffff7f` / `00000080` |
| fits `long`, truncated to `int` | `"2147483648"` | `-2147483648` | `00000080` |
| `long` overflow (positive) | `"99999999999999999999"` | `-1` | `ffffffff` |
| `long` overflow (negative) | `"-99999999999999999999"` | `0` | `00000000` |
| stdout is a pipe with no reader | `"5"` | n/a | killed by SIGPIPE |

Also covered: every single byte value 0x00–0xff as the whole input, every byte
following a digit and following a sign, 100 000-digit inputs, whitespace
padding straddling the Rust reader's 4096-byte refill boundary, stdin from
`/dev/null` and from a regular file, ignored `argv`, and a deterministic
1500-case pseudo-random fuzz plus a numeric sweep around every decimal
magnitude and integer boundary.

## Mismatches found

### 1. Exit status when stdout is a pipe with no reader (SIGPIPE)

**Symptom.** With stdout connected to a pipe whose read end is already closed,
the C program is killed by `SIGPIPE` (signal 13, shell status 141) inside
`printf`/`exit`'s flush, while the Rust program exited 0 and produced no
stderr. stdout and stderr matched (both empty); only the exit status differed.

```
$ echo 5 | ./c_src/build/driver | true                 # ${PIPESTATUS[1]} == 141
$ echo 5 | ./translation/target/release/driver | true   # ${PIPESTATUS[1]} == 0
```

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs, so a failing write returns `EPIPE` instead of terminating the process.
The translation additionally discarded the `write_all`/`flush` results, so the
error was invisible. A C program inherits the default disposition, which
terminates it.

**Fix.** `translation/src/main.rs` now restores the default `SIGPIPE`
disposition (`signal(SIGPIPE, SIG_DFL)`, declared via a plain `extern "C"`
block so no new dependency is needed) as the first thing `main` does. This is
a no-op whenever stdout is writable, and reproduces C's signal death when it is
not.

Regression coverage: `stdout_pipe_with_no_reader`. The test closes the read end
of the child's stdout *before* writing its stdin — the child blocks in
`read(0)` until then — so it is deterministic rather than racy. Reverting the
fix makes that test fail with
`C ExitStatus(unix_wait_status(13)) vs Rust ExitStatus(unix_wait_status(0))`,
which confirms the test exercises the bug.

## Behaviors deliberately preserved, not "fixed"

These are C/glibc quirks that the translation reproduces on purpose. None was a
mismatch; they are recorded because they look like bugs and a future reader may
be tempted to correct them.

- **`scanf` failure leaves `x` alone.** The return value of `scanf` is never
  checked, so on EOF or a matching failure the program prints the hex of the
  initial `0` rather than reporting an error. Empty input is therefore a
  *success* path producing `00000000` and exit 0.
- **glibc converts `%d` with `strtol`, which saturates.** An out-of-range digit
  string does not wrap: it becomes `LONG_MAX` / `LONG_MIN`, which is *then*
  truncated to `int`. So `"99999999999999999999"` yields `-1` (`ffffffff`) and
  `"-99999999999999999999"` yields `0` (`00000000`) — a wrapping accumulator
  would give different answers for both. The translation mirrors the saturate
  then truncate order.
- **Values that fit a 64-bit `long` truncate silently.** `"2147483648"` becomes
  `INT_MIN` and `"4294967296"` becomes `0`; `scanf` still reports success.
- **Host byte order is observable.** `print_hex` dumps the object
  representation of the `int`, so on x86-64 the output is little-endian
  (`42` → `2a000000`). The translation uses `to_ne_bytes`, matching the C on
  whatever host it is built for rather than hard-coding a byte order.
- **`scanf` skips whitespace across newlines**, unlike `fgets`. The leading-
  whitespace skip is unbounded and consumes `\n`, `\r`, `\t`, `\v` and `\f`, so
  a value on the fifth line is read as readily as one on the first.
- **Only the first token is consumed.** The rest of stdin, however long or
  malformed, is never examined.
- **`argv` is ignored** — `main` takes no parameters.
- **Write errors other than SIGPIPE are swallowed.** C's `exit`-time flush
  discards its error, so `> /dev/full` exits 0; the translation also ignores
  the `write_all` result. Verified identical.

## Result

`cargo test` passes (20 tests, none `#[ignore]`d, skipped or disabled) in both
the debug and release profiles. Every enumerated input produces identical
stdout, identical stderr and an identical exit status. Nothing in `c_src/` was
modified.
