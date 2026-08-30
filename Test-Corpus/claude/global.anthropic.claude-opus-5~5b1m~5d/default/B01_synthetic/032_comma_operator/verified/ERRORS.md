# Differential testing log: `c_src/src/main.c` vs. `translation/src/main.rs`

The C program is the ground truth. Both binaries were built, then run as
subprocesses over the same stdin bytes with stdout, stderr and exit status
compared byte for byte (`translation/tests/differential.rs`).

## Program under test

```c
void driver(int x) {
    for (int i = 0, j = 0; i < x; i++, j += 2)
        printf("%d %d\n", i, j);
}
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

Build / run commands:

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver` |

Both build with no errors and no warnings. Neither program ever writes to
stderr, and both always exit 0 (except on a signal, see mismatch 1).

## Mismatches found and fixed

### 1. Exit status differed when stdout is closed early (SIGPIPE)

**Symptom.** With a closed/short-read stdout pipe and enough output to fill it:

```
$ ./driver <<< 65535 | head -c 5   # C:    exit status 141 (killed by SIGPIPE)
$ ./driver <<< 65535 | head -c 5   # Rust: exit status 0
```

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. The failing `write` therefore returned `EPIPE` instead of killing the
process; the translation discards write errors (correctly, since the C ignores
`printf`'s return value), so it ran to completion and exited 0. The C program
inherits the default disposition and dies by signal 13.

**Fix.** Restore the default disposition at the top of `main`:
`signal(SIGPIPE, SIG_DFL)`. Covered by `broken_stdout_pipe_kills_both`, which
compares both `ExitStatus::code()` and `ExitStatus::signal()`.

### 2. Too much of stdin was consumed

**Symptom.** The number of bytes left on a shared stdin descriptor after exit,
observed with `{ ./driver >/dev/null; wc -c; } < input`:

| input (11 bytes) | C leaves | Rust leaves (before fix) |
|---|---|---|
| `2 REMAINDER` | 10 | 0 |
| 6 KB starting `2 R` | 6002 | 0 |

**Cause.** The translation read stdin through `io::stdin()`, which wraps fd 0 in
an 8 KiB `BufReader`. Even though the parser only *logically* consumed one byte,
the buffered reader had already drained the whole descriptor, so a later reader
of the same fd (`{ prog; cat; } < file`) saw nothing.

glibc also reads ahead into its stdio buffer, but at exit `_IO_cleanup`
repositions a *seekable* stream to the logical stream position, so the
externally visible effect is that only the bytes the conversion actually used
are consumed.

**Fix.** Read fd 0 unbuffered, one byte at a time, via a direct `read(2)` call
(retrying on `EINTR`), and emulate `scanf`'s single `ungetc` with
`lseek(0, -1, SEEK_CUR)` when a conversion is terminated by a byte rather than
by EOF. Measured against the C for every input class; the pushback rule is
uniform:

| input | bytes consumed by C | note |
|---|---|---|
| `25 X` | 2 | digits consumed, `' '` pushed back |
| `   abcdef` | 3 | whitespace consumed, `'a'` pushed back |
| `-abc` | 1 | the sign stays consumed; only `'a'` is pushed back |
| `-` | 1 | EOF after the sign: nothing to push back |
| `.5` | 0 | `'.'` pushed back |

Covered by `stdin_is_consumed_to_the_same_offset`, which hands both programs a
dup of one descriptor (dups share a file offset) and compares the final offset.

## C behaviours deliberately reproduced, not "fixed"

- **A failed `scanf` leaves `x` at its initialiser `0`**, so every matching
  failure (`abc`, `.5`, `-`, `+`, `--3`, `- 3`, a leading NUL, `\xff`) and EOF
  (empty input, whitespace only) prints nothing and exits 0. The return value of
  `scanf` is never checked, and no diagnostic is printed.
- **`%d` skips leading whitespace across newlines** — `\n\n\n3` is the same as
  `3`. The whitespace set is C-locale `isspace`: `' ' \t \n \v \f \r`.
- **Conversion stops at the first non-digit**, so `0x10` reads `0`, `1e3` reads
  `1`, `3abc` reads `3`. Only the first number is ever read: `3 5` prints 3
  lines.
- **Out-of-range values convert as a `long`, then truncate to `int`.** glibc
  saturates at `LONG_MAX`/`LONG_MIN` and the assignment to `int` keeps the low
  32 bits, so:
  `2147483648` → `INT_MIN`, `4294967296` → `0`, `9223372036854775807` → `-1`,
  and 5000 nines → `-1`. All of these print nothing because the result is
  ≤ 0. `4294967299` → `3` and does print 3 lines. The Rust reproduces the
  saturate-then-truncate order exactly rather than rejecting the overflow.
- **`x <= 0` yields no output at all**, since `i < x` fails immediately.
- **Output format** is exactly `"%d %d\n"` per line — single space, trailing
  newline on every line including the last.

## Untestable by execution

`j` overflows `int` once `i` exceeds ~2^30, which needs `x > 1073741824` and
would emit over a billion lines (tens of GB) before wrapping. The C form
(`j += 2` on a signed `int`) is UB that gcc compiles to wrapping arithmetic, and
the translation uses `wrapping_add` to match. Loop counts up to 70000 are
verified by execution, including that the final line for `x = 70000` is
`69999 139998`.

## Result

- Both programs build cleanly.
- `cargo test` in `translation/`: **20 passed, 0 failed, 0 ignored** (in both
  the debug and `--release` profiles). No test is disabled, skipped or
  `#[ignore]`d.
- An additional randomized sweep of 600 inputs over the alphabet
  `0 1 5 9 - + space \n \t \v \f \r a . x NUL \xff : / e` plus numbers with
  random surrounding noise found **0** mismatches in stdout, stderr or exit
  status.
- Nothing in `c_src/` was modified.
