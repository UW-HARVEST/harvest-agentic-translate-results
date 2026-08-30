# Differential verification: `c_src` vs `translation`

Verification of the Rust translation of `c_src/src/main.c` against the C
reference build. Both programs are compared by execution — built, fed identical
stdin, and diffed on stdout, stderr and exit status.

## Commands

| | |
|---|---|
| Build C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` |
| Run C | `./c_src/build/driver` |
| Build Rust | `cd translation && cargo build --release` |
| Run Rust | `./translation/target/release/driver` |
| Test | `cd translation && cargo test` |

Both build with no errors and no warnings. The C reference is gcc 11.5.0; the
`CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference is unoptimized
(`-O0`), which matters for the undefined behavior described below.

## What the program does

```c
void printIntPtrLine(const int *n) { printf("%d\n", *n); }
void bad()  { int *data; printIntPtrLine(data); }        // CWE-457
void good() { int data = 5; int *p = &data; printIntPtrLine(p); }

int main() {
    int x = 0;
    scanf("%d", &x);
    if (x) { good(); } else { bad(); }
    return 0;
}
```

`scanf`'s return value is **ignored**, so a matching failure or EOF leaves `x`
at its initializer `0` and falls through to `bad()`. There is exactly one
branch and two observable outputs (`5\n` or `0\n`), always with exit status 0,
always with empty stderr — but three distinct input classes reach `bad()`.

## Enumerated input classes

| Class | Reaches | Examples | Output |
|---|---|---|---|
| Parsed value non-zero | `good()` | `1`, `-42`, `+7`, `000001`, `2147483647` | `5\n` |
| Parsed value zero | `bad()` | `0`, `-0`, `+0`, `00000` | `0\n` |
| Overflow truncating to non-zero | `good()` | `2147483648`, `4294967297`, `99999999999999999999` | `5\n` |
| Overflow truncating to zero | `bad()` | `4294967296`, `8589934592`, `-99999999999999999999` | `0\n` |
| Matching failure (no assignment) | `bad()` | `abc`, `-`, `+`, `-x`, `.`, `0x10`, `3.9` | `0\n` |
| EOF before any conversion | `bad()` | empty input, whitespace-only, `/dev/null` | `0\n` |
| Leading whitespace skipped | either | `\n1`, `\t\t9`, `   \n  0` | per value |
| Trailing input ignored | either | `1abc`, `0\n1\n`, `1 2` | per first value |

## Mismatches found

### 1. `SIGPIPE` disposition — exit status differed

**Symptom.** With stdout a closed pipe, the C binary was killed by signal 13
(shell status 141) while the Rust binary exited 0.

```
printf '1' | ./c_src/build/driver           | true   # -> 141
printf '1' | ./translation/target/.../driver | true   # -> 0    (before fix)
```

**Cause.** The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs; a C
program inherits the default `SIG_DFL`. The failed `write` therefore returned
`EPIPE` to Rust — where the error was discarded — instead of terminating the
process.

**Fix.** `restore_default_sigpipe()` in `src/main.rs` resets `SIGPIPE` to
`SIG_DFL` as the first statement of `main`. Covered by
`broken_stdout_pipe_kills_both_with_sigpipe`, which closes the pipe's read end
*before* spawning the child so the first write always fails — no race.

This was the only behavioral divergence found. The items below were verified
rather than repaired, and each is pinned by a test so a regression is caught.

## Behaviors verified as already correct

### `scanf("%d")` overflow: truncation, not saturation

glibc parses `%d` into a `long`, then the assignment to `int` truncates. The
translation reproduces both halves, including the saturation glibc applies when
the digits exceed `long`:

| Input | glibc `long` | Truncated `int` | Branch | Output |
|---|---|---|---|---|
| `2147483648` | 2147483648 | `-2147483648` | `good()` | `5\n` |
| `4294967296` | 4294967296 | `0` | **`bad()`** | `0\n` |
| `99999999999999999999` | `LONG_MAX` | `-1` | `good()` | `5\n` |
| `-99999999999999999999` | `LONG_MIN` | `0` | **`bad()`** | `0\n` |

The last two are the subtle ones: overflowing in one direction flips the branch
and in the other does not. Saturating to `i32` instead of truncating — a natural
mistake in Rust — changes the output for `4294967296` and
`-99999999999999999999`. Confirmed caught by mutation (see below).

### `scanf` reads across newlines

`%d` skips *all* leading whitespace including `\n`, unlike `fgets`. So `"\n\n\n5"`
prints `5\n`, not the `bad()` output. The translation's whitespace skip uses the
full C `isspace` set.

### Matching failure performs no assignment

On `"abc"`, `scanf` returns 0 and does not write to `x`, which keeps its
initializer `0`. The translation returns `None` and leaves `x` alone rather than
writing a sentinel. A lone `-` or `+` followed by EOF is also a matching
failure, not a parse of zero — both reach `bad()`, so the distinction is not
observable here, but it is modeled correctly.

### The undefined behavior in `bad()`

`bad()` dereferences an uninitialized `int *`. This is genuine UB, so there is
no "correct" value — only the reference build's behavior. The disassembly shows
why it is nonetheless stable:

```
bad():
  sub    $0x10,%rsp
  mov    -0x8(%rbp),%rax     # never written by bad(); leftover from scanf's frame
  mov    %rax,%rdi
  call   printIntPtrLine
```

`bad()` reads a stack slot it never writes, left behind by the preceding
`scanf` call, and that slot holds a pointer to a zero word. The reference build
prints `0\n` and exits 0.

Because this depends on what `scanf` left on the stack, I checked whether it
varies with anything an input could influence, since different inputs drive
`scanf` down different internal paths:

- all three `bad()`-reaching input classes (parsed zero, matching failure, EOF)
- 20 repeated identical runs
- environment block sizes 0–5000 bytes and `argv` lengths 0–500 bytes, which
  shift the initial stack pointer
- inputs up to 200 000 digits, which push glibc's `%d` conversion onto its
  wide-input path and change its stack and allocation behavior

It printed `0\n` in every case. `UNINITIALIZED_STACK_VALUE = 0` in `src/main.rs`
records that observation, with the bug preserved: `bad()` still prints an
"uninitialized" value rather than the `5` that `good()` returns.

**Caveat worth stating plainly:** this value is a property of the reference
build, not of the C language. Rebuilt with optimization, a different libc, or a
different architecture, `bad()` could print anything or crash. The translation
matches the build in `c_src/build`, which is what is being compared.

## Test suite

`translation/tests/differential.rs` — 21 tests, roughly 800 input cases. Every
test spawns **both** binaries as subprocesses and compares stdout bytes, stderr
bytes, exit code, *and* terminating signal. Nothing is loaded as a library;
there is no `#[no_mangle]`, no `cdylib`, no `libloading`. No test is `#[ignore]`d,
skipped or disabled. The C binary is built automatically by the harness if
absent, and a build failure is fatal rather than silently skipped — comparing
against a program that did not build measures nothing.

Randomized sweeps use a fixed-seed xorshift PRNG, so any failure reproduces.

### Suite validated by mutation

A test suite that passes is only meaningful if it can fail. Five faults were
injected into the Rust source; each was caught, and the source was restored
afterwards:

| Injected fault | Result |
|---|---|
| `bad()` prints `12345` instead of `0` | caught (18 tests) |
| Overflow saturates instead of truncating | caught (5 tests) |
| Whitespace skip excludes newlines | caught (2 tests) |
| `SIGPIPE` left as `SIG_IGN` | caught (1 test) |
| Exit status 1 instead of 0 | caught (20 tests) |

The overflow and `SIGPIPE` mutations are the ones a stdout-only comparison would
miss, which is why exit code and signal are asserted on every case.

## Completion gate

- [x] Both programs build with no errors
- [x] Every enumerated input produces identical stdout, stderr and exit status
- [x] `cargo test` passes in `translation/` (21 passed, 0 failed, 0 ignored) in
      both debug and `--release`
- [x] No test disabled, skipped or `#[ignore]`d
- [x] Nothing in `c_src/` modified (only the untracked `build/` directory was
      added, by the prescribed build command)
