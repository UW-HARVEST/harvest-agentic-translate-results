# Differential verification log

C ground truth: `c_src/src/main.c` (built via `c_src/CMakeLists.txt`).
Rust under test: `translation/src/main.rs`.

Both programs are compared by **execution only** — spawned as subprocesses with
identical stdin, then stdout, stderr, exit code and termination signal are
compared byte for byte. See `translation/tests/differential.rs`.

## Commands

| | command |
|---|---|
| build C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` |
| run C | `c_src/build/driver` (reads stdin) |
| build Rust | `cd translation && cargo build --release` |
| run Rust | `translation/target/release/driver` (reads stdin) |
| test | `cd translation && cargo test` |

## What the C program does

```c
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

`driver` memcpy's the 4 bytes of `x` into a `char[4]` and `print_hex` prints each
byte as `%02x` followed by `"\n"`. Exit status is always 0 and stderr is always
empty. All behavioural branching therefore lives in `scanf("%d", &x)`:

1. **EOF / whitespace-only input** → no conversion, `x` keeps its initialiser `0`
   → `00000000`.
2. **Matching failure** (first non-space token does not start a valid integer)
   → `x` is left *untouched*, again `0`. This is the "looks like a bug" path:
   the return value of `scanf` is never checked, so bad input is
   indistinguishable from an input of `0`.
3. **Successful conversion** → glibc converts via `strtol` into a `long`,
   saturating at `LONG_MAX`/`LONG_MIN` on overflow, then assigns to `int`,
   truncating to the low 32 bits.

## Mismatches found

### 1. `SIGPIPE`: Rust exited 0 where C is killed by signal 13 — FIXED

**Symptom.** With stdout connected to a pipe whose reader has already closed:

```
c_src/build/driver              -> code=None,    signal=Some(13)
translation/.../driver (before) -> code=Some(0), signal=None
```

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` before
`main` runs. A C program inherits the default disposition (`SIG_DFL`), so its
`printf`/flush to a dead pipe kills the process. The Rust version instead got
`EPIPE` from `write_all`, which the existing `let _ = ...` discarded, and fell
through to a normal `return 0`.

**Fix.** `restore_default_sigpipe()` in `src/main.rs` calls
`signal(SIGPIPE, SIG_DFL)` as the first statement of `main`, declared via a
direct `extern "C"` block so no new crate dependency is needed.

**Regression test.** `stdout_pipe_with_dead_reader_matches`. Verified by
mutation: commenting out the `restore_default_sigpipe()` call makes that test
fail with exactly the diff above, and restoring it makes it pass — so the test
is load-bearing, not decorative.

### 2. No other mismatches

Every other input class agreed on the first run. The following behaviours were
checked and confirmed to already match, rather than assumed:

- **`scanf` reads across newlines.** `"\n\n\n7\n"` yields `07000000`; leading
  whitespace of every C kind (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`) is skipped.
  The Rust `Stdin` reader replicates this instead of using line-oriented input,
  which is what a naive `read_line` translation would have got wrong.
- **Overflow saturates in `long`, then truncates to `int`** — not wrapping
  arithmetic in 32 bits, and not a parse error:
  - `99999999999999999999` → `LONG_MAX` → low 32 bits `0xffffffff` → `ffffffff`
  - `-99999999999999999999` → `LONG_MIN` → low 32 bits `0x00000000` → `00000000`
  - `9223372036854775808` → `ffffffff`; `-9223372036854775809` → `00000000`
  - Note the asymmetry: a huge *negative* input prints the same bytes as `0`.
- **In-range truncation** past `INT_MAX`: `2147483648` → `00000080`,
  `4294967296` → `00000000`, `4294967297` → `01000000`.
- **Partial matches consume only the digit prefix**: `12abc` → `0c000000`,
  `1e3` → `01000000`, `0x1f` → `00000000` (the `0` converts, `x` stops it).
- **Sign handling**: `+5` converts; a lone `-`, `+`, `- 5`, `--5`, `+-5` are all
  matching failures giving `00000000`.
- **Byte-level input hazards**: a leading NUL or `0xff` byte is a matching
  failure, not a crash or a skipped character.
- **`%02x` zero padding** for every one of the 4 byte positions, and
  little-endian byte order (`1` → `01000000`, not `00000001`).
- **Only the first conversion happens**: `10 20` → `0a000000`; trailing garbage
  lines are ignored.
- **Failing writes to an open stdout** (`/dev/full`, ENOSPC): both exit 0 with
  empty stderr, because neither program checks `printf`'s return value.

## Coverage

`cargo test` runs 16 tests covering ~250 distinct inputs, including empty input,
whitespace-only input, one item, `INT_MAX`/`INT_MIN`, `LONG_MAX`/`LONG_MIN` and
one past each, digit strings of 5 000 and 10 000 characters, every matching
failure form, all 32 powers of two and their negations, and the exhaustive range
`-40..=40`. No test is `#[ignore]`d, skipped or disabled.

Beyond the committed suite, a throwaway fuzz sweep of **6 000** inputs (random
byte strings over a digit/sign/whitespace/punctuation/NUL/`0xff` alphabet, plus
random integers of 1–25 digits with random trailing bytes) produced **0**
mismatches in stdout, stderr and exit status.

Nothing in `c_src/` was modified; only the untracked `c_src/build/` output
directory was created, as the build instructions direct.
