# Differential testing: mismatches found and fixed

The C reference is `c_src/` (`src/main.c` + `src/sillymain.c`), built with CMake
into `target-c-reference/driver`. The Rust program is this crate's `driver`
binary. Both are driven as subprocesses by `tests/differential.rs`, which
compares stdout, stderr and the wait status (exit code *and* terminating signal)
for every input.

## What the C program actually does

```c
int main() { return helloworld(); }

int helloworld() {
    printf("Hello World!\n");
    return 0;
}
```

It reads nothing — no `scanf`, `fgets`, `getchar` or `read` — and never touches
`argv` or the environment. So there is no data-dependent branch to enumerate:
the only observable behaviour is the fixed 13-byte write `Hello World!\n` on
stdout, an empty stderr, and exit status 0. `printf`'s return value is discarded,
so a *failed* write must still produce exit status 0.

That makes the input classes environmental rather than textual: what stdin holds
(nothing / one line / no trailing newline / many lines / more than a pipe buffer
/ non-UTF-8 / already at EOF), what argv holds (none / one / many / non-ASCII),
and what stdout is connected to (pipe, regular file, closed descriptor,
`/dev/full`, merged with stderr, a pipe whose reader has gone away).

## Mismatch 1 — exit status on a broken pipe (found, fixed)

**Symptom.** When stdout is a pipe whose read end has already been closed, the
two programs disagreed on the wait status:

| Program | Wait status |
|---|---|
| C | killed by signal 13 (`SIGPIPE`) — a shell reports `141` |
| Rust (before the fix) | normal exit, code `0` |

Reproduced from a shell before the fix:

```
$ { sleep 0.3; ./c_src/build/driver; echo "EXIT=$?" >&2; } | head -c 0
EXIT=141
$ { sleep 0.3; ./translation/target/release/driver; echo "EXIT=$?" >&2; } | head -c 0
EXIT=0
```

**Cause.** A C program inherits `SIGPIPE` at `SIG_DFL`, so the `write(2)` behind
`printf` raises `SIGPIPE` and the default disposition terminates the process. The
Rust standard library installs `SIG_IGN` for `SIGPIPE` in its pre-`main` runtime
setup, precisely so that Rust programs see an `EPIPE` error instead of dying. The
translation then discarded that error — correctly mirroring the C code's habit of
ignoring `printf`'s return value — and returned 0. The result was a program that
ignored a condition the C program dies from.

Note this is *not* a case of the C code being buggy and the Rust code being
"better": the requirement is byte-and-status identity, and 141 is the status the
C program produces.

**Fix.** `src/main.rs` now restores the C default disposition as the first thing
`main` does, before any write, via a direct `extern "C"` declaration of `signal`
(no `libc` dependency, keeping the crate dependency-free):

```rust
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

This is inert in normal operation — with no broken pipe the disposition is never
consulted — so it only affects the divergent case.

**Test.** `broken_pipe_kills_the_process_with_sigpipe`. It spawns
`sh -c "sleep 1; exec <bin>"` with stdout piped, drops the read end while the
child is still sleeping, then compares statuses. The test also asserts the C
reference really is killed by signal 13, so it cannot pass vacuously if the
broken pipe fails to materialise.

**Negative control.** With the `restore_default_sigpipe()` call commented out,
that test fails with "exit status differs on a broken pipe"; with it restored,
all tests pass. So the test detects the defect rather than merely coexisting with
the fix.

## Verified-identical behaviour (no mismatch found)

Each of these was checked for stdout, stderr and wait status. They are recorded
because "no difference" is only meaningful if the case was actually exercised.

| Input class | Result |
|---|---|
| empty stdin, no arguments | identical — `Hello World!\n`, empty stderr, exit 0 |
| one line on stdin (`1\n`) | identical; unread input is ignored by both |
| input without a trailing newline (`42`) | identical — neither program reads stdin, so `scanf`/`fgets` newline semantics never come into play |
| 1000 lines on stdin | identical |
| 256 KiB on stdin (past the pipe capacity, so the writer blocks and then sees `EPIPE`) | identical; the test tolerates the parent's failed write, which is not part of either program's output |
| non-UTF-8 and NUL bytes on stdin | identical; nothing decodes stdin |
| stdin already at EOF (writer dropped before wait) | identical |
| 1 argument, 5 arguments, 500 arguments, an empty-string argument, `--help`/`--version`-looking arguments, arguments with spaces/tabs/non-ASCII | identical — `int main()` declares no parameters and argv is never read |
| stdout to a regular file (C stdio uses full buffering here, not the pipe's) | identical file contents, `Hello World!\n`; confirms the stream is flushed in both |
| stdout closed before `exec` (`>&-`) | identical — the write fails, both ignore it, both exit 0 |
| stdout to `/dev/full` (write fails with `ENOSPC`) | identical — both exit 0, matching the discarded `printf` return value |
| stdout and stderr merged (`2>&1`) | identical combined stream, `Hello World!\n`; pins down that stderr receives nothing and write ordering matches |
| empty environment (`env_clear`) | identical; no `getenv` in the C |
| `LC_ALL`/`LANG` set to `C`, `POSIX`, `en_US.UTF-8`, `de_DE.UTF-8`, `tr_TR.UTF-8`, and empty | identical; the format string has no conversion specifiers, so no locale-sensitive formatting exists |
| 25 repeated runs | identical every time, and identical to each other — output is deterministic |

## Things deliberately not "fixed"

* The `printf` return value is discarded in the C, so the Rust discards the
  results of `write_all` and `flush`. A write failure must not become a non-zero
  exit status.
* `helloworld` returns `int` and `main` forwards it as the process status, so
  `src/main.rs` uses `std::process::exit`. Because that skips Rust's
  flush-at-exit, `sillymain.rs` flushes stdout explicitly before returning.
* No trailing-whitespace, spacing or precision differences exist to reproduce:
  the format string is a plain literal with no conversions.

## How to reproduce

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # C reference
cd translation && cargo build --release                                # Rust
cd translation && cargo test                                           # differential suite
```

`cargo test` builds the C reference itself, into `translation/target-c-reference`
rather than `c_src/build`, so running the suite never writes inside `c_src/`.
