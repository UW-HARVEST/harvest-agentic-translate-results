# Differential verification: mismatches found and fixed

Ground truth: `c_src/src/main.c` (never modified). Comparison method: build both
programs, run them as subprocesses over the same scenarios, diff stdout, stderr
and exit status. Tests live in `translation/tests/differential.rs`.

## The C program's branch structure

```c
int main() {
    printf("Hello World!\n");
    return 0;
}
```

There is exactly one path: no conditionals, no loops, no early returns, no
`scanf`/`fgets`, no use of `argc`/`argv`, no `errno` checks, and no inspection of
`printf`'s return value. Consequently the input classes are not textual — they
are process-level: stdin contents (always ignored), argv (always ignored), the
environment, and the state of the stdout/stderr descriptors, which determines
whether the buffered flush at exit succeeds, fails silently, or kills the
process.

## Mismatch 1 — exit status on a broken stdout pipe (fixed)

| | stdout | stderr | status |
|---|---|---|---|
| C | empty | empty | killed by signal 13 (`SIGPIPE`) |
| Rust (before fix) | empty | empty | exited 0 |

Reproduction: spawn the program with stdout on a pipe and close the read end
before the child writes.

```python
pr = subprocess.Popen([prog], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
pr.stdout.close()
pr.wait()   # C: -13,  Rust before fix: 0
```

Cause: the Rust standard library installs `SIG_IGN` for `SIGPIPE` during runtime
startup, before `main` runs. A C program started via `exec` inherits the default
disposition (`SIG_DFL`), so its `write` during the exit-time stdio flush raises
`SIGPIPE` and terminates the process. In Rust the write instead returned
`EPIPE`, which `main.rs` discards, so the program exited 0. Shell-visible
statuses therefore differed: 141 vs 0.

Fix (`translation/src/main.rs`): restore the default `SIGPIPE` disposition as the
first action in `main`, via a direct `signal(2)` FFI declaration (no new
dependency — libc is already linked). After the fix both programs report signal
13 with empty stdout and stderr.

Regression test: `stdout_pipe_read_end_closed_before_write`.

## Behaviors confirmed identical (no fix needed)

These were checked because they are the places a translation typically drifts,
not because they failed.

- **stdout bytes.** Exactly `Hello World!\n` (13 bytes, `4865 6c6c 6f20 576f 726c 6421 0a`),
  one trailing newline, no other whitespace. Verified byte for byte, not via
  string comparison.
- **stderr.** Empty in every scenario.
- **Exit code.** 0 in every non-signal scenario.
- **stdin is never consumed.** Empty pipe, `/dev/null`, a pipe closed without
  writing, one line, a line with no trailing newline, only newlines,
  whitespace/CRLF, non-numeric text, integers past `INT_MAX`/`INT_MIN` and past
  `u64`, NUL bytes and invalid UTF-8, 1 MiB of lines, and a single 300 000-byte
  line all produce identical output. A translation that used a reader would have
  diverged here; this one performs no reads.
- **argv is never inspected.** Zero args, one arg, flag-like args (`-h`,
  `--help`, `--version`, `-`, `--`), empty and embedded-newline and non-ASCII
  args, and 1024 args all behave the same.
- **Environment and locale are irrelevant.** Cleared environment, `LC_ALL=C`,
  and `LC_ALL=tr_TR.UTF-8` with `LC_NUMERIC=de_DE.UTF-8` all produce the same
  ASCII literal.
- **stdout to a regular file.** Same 13 bytes; the change in stdio buffering mode
  (fully buffered file vs pipe vs tty) does not alter content or ordering.
- **stdout closed (`>&-`).** The write fails; neither program reports an error
  and both exit 0, because the C code ignores `printf`'s return value.
- **stderr closed (`2>&-`).** Unaffected; stdout still correct, both exit 0.
- **stdout on `/dev/full`.** Write fails with `ENOSPC`; both still exit 0 with
  empty stderr — again because the C return value is unchecked.
- **Working directory.** Running from a different cwd changes nothing.
- **Determinism.** Ten consecutive runs produce identical stdout, stderr and
  status for both programs.

## Status

Both programs build cleanly, `cargo test` passes 29 differential tests with none
`#[ignore]`d, skipped or disabled, and `c_src/` sources are untouched (only the
out-of-tree `c_src/build/` directory was created, as the build instructions
require).
