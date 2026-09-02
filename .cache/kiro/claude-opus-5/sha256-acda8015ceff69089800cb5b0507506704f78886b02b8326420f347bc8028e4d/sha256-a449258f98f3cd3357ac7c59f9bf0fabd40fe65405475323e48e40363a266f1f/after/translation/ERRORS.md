# Differential-testing log: `c_src` (ground truth) vs `translation`

Programs compared as executables, never as libraries:

- C: `c_src/build/driver` — built with `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
- Rust: `translation/target/release/driver` — built with `cd translation && cargo build --release`
- Tests: `cd translation && cargo test` (`tests/differential.rs`, `tests/oob_characterization.rs`,
  harness in `tests/common/mod.rs`; each case runs both binaries as subprocesses and
  compares stdout, stderr and exit status)

The C program under test:

```c
int foo(const char *in, char c) {
    int res = 0;
    for (const char *s = in; s = strchr(s, c); s++) res++;
    return res;
}
void driver(const char *in) {
    printf("A: %d\n", foo(in, 'A'));
    printf("x: %d\n", foo(in, 'x'));
}
int main() {
    char in[1000] = "";
    fread(in, 1, sizeof(in), stdin);
    driver(in);
    return 0;
}
```

It has no error path: it never writes to stderr, and `main` always returns 0. The
only ways its observable behaviour varies are the two entries below.

---

## Mismatch 1 — exit status when stdout's reader is gone (FIXED)

**Symptom.** With stdout connected to a pipe whose read end is already closed:

| program | exit status | stdout | stderr |
|---|---|---|---|
| C | killed by signal 13 (`SIGPIPE`) | empty | empty |
| Rust (before fix) | exited 0 | empty | empty |

Reproduced 20/20 runs in both directions, so this was deterministic, not a race.

**Cause.** The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs. A
failed write therefore returns `EPIPE`, the return value is discarded exactly as
C discards `printf`'s, and the process exits 0. The C program inherits the
default disposition and is killed by the signal instead.

This is precisely the failure the task description warns about: stdout was
byte-identical (empty in both cases), so a stdout-only comparison passed while
the exit status disagreed.

**Fix.** `translation/src/main.rs` now calls `restore_default_sigpipe()` as the
first statement of `main`, declaring `signal` from libc directly (no new
dependency) and installing `SIG_DFL` for `SIGPIPE`.

**Related check, no change needed.** With file descriptor 1 *closed* rather than
piped (`exec ./driver >&-`), `printf` fails with `EBADF`, no signal is raised and
both programs exit 0 — already matching.

**Also changed while fixing this.** The Rust program now formats both lines into
one buffer and issues a single `write_all`, matching C stdio's full buffering on
a non-terminal stdout (glibc flushes both lines together at exit). Previously the
line-buffered `Stdout` wrapper emitted `A: …` and `x: …` as separate writes, so a
reader that disappeared between the two lines would have received different bytes
from the two programs.

**Regression check.** Commenting out the `restore_default_sigpipe()` call makes
`differential::stdout_reader_gone` fail with "exit status differs with a dead
stdout reader"; restoring it makes it pass. Verified, then reverted.

---

## Mismatch 2 — the C program reads out of bounds on a completely full buffer (not fixable in Rust; test strategy documented)

**Symptom.** For any input of >= 1000 bytes whose first 1000 bytes contain no NUL,
the C program's output is not a function of its input. Tallying 600 runs of the C
binary on the same 1000-byte input (`'A' * 1000`):

```
A: 1000  x: 0   -> 590 runs   (the in-bounds answer)
A: 1000  x: 1   ->   7 runs
A: 1001  x: 0   ->   3 runs
```

The Rust program prints `A: 1000 / x: 0` on 100% of runs.

This is why the suite was briefly flaky: `length_boundaries_all_a` failed once
with C reporting `x: 1`, then passed on re-run.

**Cause.** From the disassembly of the built binary:

```
main:  sub $0x3f0,%rsp          ; frame is 1008 bytes
       ...                      ; zero-fills exactly 0x3e8 = 1000 bytes at rbp-0x3f0
       mov $0x3e8,%edx ; call fread
```

`in` lives at `rbp-0x3f0` and is 1000 bytes long, so it ends exactly at `rbp-8`.
`char in[1000] = ""` zero-fills precisely those 1000 bytes. When `fread` fills all
of them and none is NUL, nothing inside the buffer terminates the string, so
`strchr` keeps scanning into:

1. the 8 bytes of frame padding at `rbp-8 .. rbp-1`, which are never written by
   this program (leftovers from the dynamic loader and libc startup), and then
2. the saved frame pointer at `[rbp]` — a stack address, randomised by ASLR on
   every execution.

The scan stops at the first zero byte it finds there. Whether an `0x41` ('A') or
`0x78` ('x') appears first depends on those bytes, i.e. on ASLR and on residue the
input does not control. The observed ~1.2% rate for one extra match matches what
you would expect from roughly three random bytes (3/256).

**Why the Rust program cannot reproduce it.** The extra matches come from
uninitialised memory and a randomised address inside the C process. No
deterministic Rust program can predict them, and the C binary does not agree with
*itself* across runs.

**What the Rust program does instead.** It produces the in-bounds answer, which is
the C program's output on ~98% of runs and its only stable behaviour.

**How this is tested.** Inputs are split by whether the C program stays inside its
buffer (`common::reads_out_of_bounds`):

- fewer than 1000 bytes read, or a NUL inside the first 1000 bytes → strict
  byte-for-byte comparison of stdout, stderr and exit status
  (`assert_same`)
- 1000+ bytes with no NUL → `assert_same_modal`, which runs the C program 15 times,
  takes its modal stdout, and requires the Rust program to equal it and to be
  deterministic; stderr and exit status are still compared strictly on every run

`tests/oob_characterization.rs` pins the finding down directly: the Rust output
never varies over 60 runs, the C program's modal output is the in-bounds answer,
and its deviations only ever *over*-count (out-of-bounds bytes can add matches,
never remove them). It also asserts that a 999-byte input is fully deterministic
in the C, confirming the buffer boundary is the trigger.

---

## Checked and already correct (no change required)

Verified over the enumerated cases plus a 300-case seeded fuzz and a separate
3000-run ad-hoc sweep:

- **`fread` is not line oriented.** `A\nx\nAAxx\n` yields `A: 3 / x: 3`; the read
  continues across newlines, unlike `fgets`/`scanf`.
- **Zero fill supplies the terminator.** For 0..999 bytes read, `in[filled]` is
  still 0, so the C string is exactly the bytes read.
- **Embedded NUL truncates.** `A\0x` gives `A: 1 / x: 0`; everything from the first
  NUL on is invisible to `strchr`, including a NUL at byte 0.
- **Truncation at 1000 bytes.** Input beyond `sizeof(in)` is discarded: matches at
  byte 1001 and later are never counted, and the writer's `EPIPE` when the child
  exits early is harmless.
- **`s++` after each hit.** Adjacent matches (`AA`, `AxAxAx`) and a match on the
  final byte are counted correctly; after the last match `s` lands on the NUL and
  the next `strchr` returns NULL.
- **Case and byte sensitivity.** `a` and `X` are not counted; bytes >= 0x80 are
  irrelevant because the searched-for `'A'`/`'x'` are positive in a signed `char`.
- **Output format.** `"A: %d\n"` then `"x: %d\n"`, in that order, with a trailing
  newline and no stderr output.
- **`argv` is ignored.** `main` takes no parameters; extra arguments change nothing.
- **Unreadable stdin.** With a directory as stdin, `fread` fails, `in` keeps its
  zero fill, and both programs print `A: 0 / x: 0` and exit 0.
- **Slowly arriving stdin.** Feeding input in 1-, 137-, 250- and 300-byte chunks
  keeps both programs' read loops going until the buffer is full or EOF.
- **Integer width.** `res` is a C `int`, but it is bounded by 1000, so no overflow
  is reachable; the Rust side still uses a wrapping `i32` add.

## Completion status

- both programs build without errors
- `cargo test` in `translation/`: 24 tests, all passing; 8 consecutive full-suite
  runs were green (checked after the flakiness in Mismatch 2 was understood)
- no test is `#[ignore]`d, skipped or disabled; the two `#[cfg(unix)]` tests gate
  Unix-only fd manipulation and do run on this platform
- nothing in `c_src/` was modified (only the untracked `c_src/build/` output
  directory required by the documented build command was created)
