# VERIFICATION.md — what was checked, what was wrong, how to re-run

## The program

`c_src/src/main.c` is a 14-statement C translation unit written with digraphs and
the `<iso646.h>` operator spellings. De-sugared:

```c
void driver(int x, int y) { int result = x | ~y; printf("%d", result); puts(""); }
int main() { int x = 0, y = 0; scanf("%d", &x); scanf("%d", &y); driver(x, y); return 0; }
```

There is **not a single branch** in the code, so essentially all of the
behavioural surface is inherited from glibc's `scanf("%d")` — its whitespace
skipping, sign handling, push-back-on-failure rule, and the `strtol`-then-truncate
conversion path — plus the process-level defaults a C program starts with.

`CMakeLists.txt` builds an **executable**, so the primary comparison boundary is
the process: identical stdin bytes must produce identical stdout, stderr, exit
code and terminating signal. `main.c` also defines two external symbols
(`driver`, `main`), so it is additionally compiled with `-shared -fPIC` and
compared against a Rust `cdylib` through `dlopen`/`libloading`.

## Defects found and fixed

Both were found by probing the C behaviour rather than by reading the Rust code,
and both are invisible to happy-path testing.

### 1. stdin was consumed eagerly instead of lazily

The translation opened with `std::io::stdin().read_to_end(&mut data)`. `scanf` is
lazy: it reads only what its directives need. Consequences of the difference:

| stdin | C | Rust (before) |
|-------|---|---------------|
| `yes 5 \|` (never reaches EOF) | exits instantly | 3–4 s, multi-GB allocation, only terminating because `read_to_end` eventually returned `ErrorKind::OutOfMemory` |
| `cat /dev/zero \|` | exits instantly | same |

On a memory-capped host the eager version would have been OOM-killed (a different
exit status), and it consumed gigabytes where C consumes one 4 KiB buffer.

**Fix** (`src/lib.rs`): the scanner now sits directly on `BufRead::fill_buf` /
`consume`, which is exactly the peek-then-commit primitive `scanf` implements with
`ungetc`, and keeps reads lazy and allocation-free.

**Regression test**: `cfg30_unbounded_stdin` feeds an endless stream and asserts
each build consumes **< 1 MiB** of it. Byte count rather than wall-clock time is
deliberate — see the note on the negative control below.

### 2. `SIGPIPE` was left ignored

The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs; a C program
starts with `SIG_DFL`. Writing to a pipe with no reader therefore diverged:

| | C | Rust (before) |
|---|---|---|
| exit status | killed by signal 13 (shell status 141) | exited **0** |

**Fix** (`src/lib.rs`, `src/main.rs`): `restore_default_sigpipe()` puts `SIGPIPE`
back to `SIG_DFL` at the top of `main`. It is deliberately *not* called by the
`.so`'s `main` export, because the C `main` does not touch signal dispositions
either — the reset exists purely to undo the Rust runtime's start-up work in the
executable.

**Regression test**: `err18_stdout_epipe_sigpipe` creates a pipe, closes the read
end *before* spawning (so `EPIPE` is deterministic rather than racy) and asserts
both builds report `signal == Some(13)`.

## Structural changes

`src/main.rs` was a single file; it is now split so the same code can back both
the executable and a shared object:

| file | role |
|------|------|
| `src/lib.rs` | the translation: `Scanner` (the `scanf("%d")` model), `driver_impl`, `c_main`, `restore_default_sigpipe` |
| `src/main.rs` | process entry point — restores `SIGPIPE`, calls `c_main` |
| `examples/driver_ffi.rs` | `cdylib` exporting `#[no_mangle] driver` and `main`, mirroring the C `.so`'s symbol table |
| `build.rs` | compiles `c_src/src/main.c` (read-only) into `libc_driver.so`, `c_driver` (`-O2`) and `c_driver_O0` |

Nothing in `c_src/` was modified.

## Two harness bugs that caused false passes

Worth recording, because each one made a broken build look correct:

1. **Stale artifacts.** `cargo test --test <name>` does not rebuild `example`
   targets, so the FFI tests `dlopen`ed a leftover `.so` and passed against an
   injected `^`-instead-of-`|` bug. Separately, a `target/release/driver` from an
   earlier build made the `SIGPIPE` row pass while the fix was absent.
   `tests/common/mod.rs::assert_fresh` now hard-fails when any artifact predates
   its sources.
2. **Captured-descriptor interleaving.** Comparing `printf` output means
   redirecting file descriptor 1, which is process-wide; the test harness writes
   its own progress lines to it from other threads, corrupting the capture. Every
   fd-capturing test now runs in a dedicated single-threaded subprocess
   (`ffi_test!`).

## Negative control (why the check marks mean something)

`scripts/negative_control.sh` injects 11 realistic translation bugs one at a time
and requires the suite to reject each one:

```
KILLED bitwise OR becomes XOR                          KILLED strtol clamp done at int width
KILLED bitwise NOT dropped                             KILLED long->int truncation -> saturation
KILLED operands swapped                                KILLED digit test accepts letters too
KILLED trailing newline from puts("") dropped          KILLED stdin slurped eagerly
KILLED SIGPIPE left ignored by the Rust runtime        KILLED vertical tab not whitespace
KILLED leading '+' no longer consumed
All 11 mutants were rejected.
```

The eager-stdin mutant **survived** the first run: `cfg30` was asserting
wall-clock time `< 5 s` and the mutant finished in 3–4 s. That is why the
assertion is now on bytes consumed, where the gap between lazy and eager is about
six orders of magnitude instead of a factor of two.

## Scope

| axis | count | notes |
|------|-------|-------|
| Cargo feature combinations | 1 | `[features]` is absent; `scripts/verify.sh` derives the power set rather than hard-coding it |
| C preprocessor / CMake options | 0 | no `#if`, no `option()` |
| Cargo profiles verified | 2 | dev and release (`panic = "abort"`) |
| tests | 68 | 37 process rows, 18 error rows, 9 FFI rows, 4 symbol-parity |
| exported symbols compared | 2 / 2 | `driver`, `main` — `nm -D` diff is empty |

## Re-running

```bash
# Everything: CMake build, feature power set, both profiles, full suite.
scripts/verify.sh

# Skip the release pass.
scripts/verify.sh --quick

# Prove the suite can still fail.
scripts/negative_control.sh

# Plain run. Use *unfiltered* `cargo test`: a `--test <name>` filter skips the
# example cdylib, and the staleness guard will refuse to run rather than compare
# against an out-of-date .so.
cargo test
```
