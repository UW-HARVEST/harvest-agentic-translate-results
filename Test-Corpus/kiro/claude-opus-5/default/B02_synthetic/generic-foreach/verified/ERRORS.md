# Differential verification: C vs. Rust `driver`

Ground truth is `c_src/`. The Rust crate in `translation/` must produce
byte-identical stdout, byte-identical stderr and the same exit status for every
input. Nothing in `c_src/` was modified; the only addition there is the
`build/` directory that CMake produces.

## How it is verified

Both programs are run as subprocesses and diffed. No Rust code is loaded as a
library.

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver` |
| Tests | `cd translation && cargo test` |

`translation/tests/harness/mod.rs` builds the C binary on demand, feeds stdin
from a dedicated thread and drains stdout/stderr concurrently (so multi-megabyte
output cannot deadlock a pipe), and normalises the exit status into
`(code, signal)` so a signal death is never confused with an exit code.

The Rust binary under test defaults to `CARGO_BIN_EXE_driver`. Set
`RUST_DRIVER_BIN` to aim the same suite at another artefact; the suite was run
against both the debug and the `--release` binary, with identical results.

## Mismatches found

### 1. `SIGPIPE`: Rust exited 0 where C died with signal 13

**The only genuine behavioural mismatch found.**

Reproduction — a session whose output exceeds the pipe buffer, with a consumer
that stops reading:

```sh
python3 -c "print('6\n'*200)" > /tmp/big.txt
( c_src/build/driver              < /tmp/big.txt; echo "rc=$?" >&2 ) 2>&1 | head -c 100 >/dev/null
( translation/target/release/driver < /tmp/big.txt; echo "rc=$?" >&2 ) 2>&1 | head -c 100 >/dev/null
```

| | before the fix | after the fix |
|---|---|---|
| C | `rc=141` (killed by `SIGPIPE`) | `rc=141` |
| Rust | `rc=0` | `rc=141` |

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` before
`main` runs, so a failed write to a closed stdout comes back as an `EPIPE`
`io::Error`. Every write in the translation is `let _ = write!(...)`, mirroring
C's habit of ignoring `printf`'s return value, so the error was swallowed and the
program ran to completion and exited 0. The C program has no such handler
installed: the first blocked write is fatal and the shell reports `128 + 13 =
141`.

Note that stdout is *identical* in this scenario — only the exit status differs.
A test comparing stdout alone would have passed.

**Fix.** `restore_default_sigpipe()` in `translation/src/main.rs` resets
`SIGPIPE` to `SIG_DFL` as the first statement of `main`, restoring the C default
disposition. It is `#[cfg(unix)]`-gated with a no-op fallback.

**Regression test.** `differential.rs::broken_stdout_pipe_matches` reads 64 bytes
of stdout, drops the read end, and asserts the C program died from signal 13
*and* that the Rust program's status matches. The first assertion keeps the test
from going vacuous if the environment ever stops delivering `SIGPIPE`.

## Behaviours deliberately preserved, including the odd-looking ones

Checked against the C source and confirmed by the passing suite. None of these
required a change — they are recorded so a reader can re-verify them.

- **`max_price` seeds from `0.0`, `min_price` from the first element**
  (`inventory.c`, `calculate_inventory_stats`). Asymmetric, and it would report
  `0.00` as the most expensive item for an all-negative-price inventory. Copied
  verbatim in `inventory.rs`; not reachable from `main`, whose inventory is
  hard-coded.
- **`min_order` sentinel of `-1.0`** in `calculate_order_stats`, so an order with
  a negative total is treated as "unset". Copied verbatim.
- **`fgets(input, 256, stdin)` does not read across newlines**, and a line longer
  than 255 bytes is delivered as several separate "lines", each validated on its
  own. A 255-character line leaves its `'\n'` in the stream, which then becomes
  an empty line and prints `Invalid input`. `cio.rs::Stdin::fgets` reads
  byte-at-a-time over an internal buffer so that exactly the bytes C consumes are
  consumed, leaving the remainder for the next call.
- **`fgets` copies NUL bytes into the buffer**, and `sscanf` then stops at the
  first one. `"\0" "6"` is `Invalid input`, not demo 6. `sscanf_int` truncates its
  argument at the first NUL for this reason.
- **`sscanf("%d")` overflow.** glibc collects the digit run and converts it with
  `strtol` — saturating at the `long` bounds — then assigns the low 32 bits to an
  `int`. So `4294967303` selects Exit, `2147483648` becomes `INT_MIN`, and
  anything past `LONG_MAX` becomes `-1`. `sscanf_int` accumulates in `i64`,
  clamps to `i64::MIN`/`i64::MAX` on overflow, then does `as i32`.
- **Validation order in `main`**: `fgets` NULL check first (break), then the
  `sscanf != 1` check (`Invalid input`, `continue`), then the `switch`
  (`Invalid choice` in `default`). `case 7` is the only early `return 0`, and it
  leaves the rest of stdin unread.
- **`printf` formatting**: `%.2f`, `%.1f`, `%zu`, `%lld`, the `°` (U+00B0) in the
  temperature output, the 40-character `═` runs in the banner, `"Choice: "` with
  no trailing newline, and the blank line `print_item` callers emit in demo 3.
  The C program never calls `setlocale`, so it stays in the `"C"` locale and the
  decimal separator is always `.` regardless of environment.
- **Integer accumulators** use `wrapping_add`/`wrapping_mul` (`sum` as `int`,
  `product` as `long long`, `total_items` as `int`) so a debug build cannot panic
  where C would wrap.
- **Null-pointer guards** (`if (!items || !category) return;`) are omitted in the
  Rust, which passes references. Unreachable from `main`, which always supplies
  live containers, and therefore not observable.

## Coverage

Input classes enumerated from `main.c` and covered by `tests/differential.rs`:

- empty input; `/dev/null` stdin; file descriptor 0 closed (`fgets` fails with
  `EBADF` rather than seeing EOF)
- a single newline; 50 consecutive blank lines
- input with no trailing newline (valid choice, Exit, and invalid)
- EOF reached after a demo, without ever selecting Exit
- each of `case 1` … `case 6`; `case 7` with unread input still queued
- `default:` — `0`, `8`, `9`, `-1`, `-0`, `100`, `INT_MAX`, `INT_MIN`
- `Invalid input` — `abc`, empty, spaces, tab, `-`, `+`, `--3`, `.5`, `x1`, `#`,
  `/`, `\v`, `\f`
- `%d` leading whitespace, `+`/`-` sign, leading zeros, trailing junk, `0x10`,
  `1e5`
- `%d` truncation and `strtol` saturation (12 boundary values around `INT_MAX`,
  `INT_MIN`, `LONG_MAX`, `LONG_MIN`)
- `\r\n` line endings and a lone `\r`
- embedded NUL bytes in four positions; 260 consecutive NULs
- non-ASCII and invalid-UTF-8 bytes
- the `char input[256]` boundary at 254, 255, 300, 400 and 1000 bytes, with and
  without newlines
- repeated demos, to confirm no state carries between runs
- stderr empty and exit code 0 asserted absolutely, not just relatively
- `SIGPIPE` on a closed stdout consumer
- a ~2.8 MB session: 500 full demo runs plus a 100 KB single line

`tests/fuzz.rs` adds 510 seeded pseudo-random cases (fixed seeds, so failures
reproduce) across four generators: token-built menu sessions, uniform random
bytes, digit-and-whitespace-heavy bytes, and long many-demo sessions.

28 tests total. None is `#[ignore]`d, skipped or disabled.
