# Differential verification: C vs. Rust

Ground truth: `c_src/src/main.c` (never modified). The Rust binary must produce
byte-identical stdout, byte-identical stderr and an identical exit status.

## How to run each program

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./c_src/build/driver < input

# Rust
cd translation && cargo build --release
./translation/target/release/driver < input
```

Differential suite: `cd translation && cargo test` (21 tests, none `#[ignore]`d,
none skipped). The suite builds the C binary via CMake on first use and drives
both programs as subprocesses over pipes.

## What the program does

```c
int main() {
    int x = 0;
    scanf("%d", &x);
    if (x) good(); else bad();   // good() -> "string\n";  bad() -> uninit ptr
    return 0;
}
```

Only two observable outputs exist (`"string\n"` and `"\n"`), always with empty
stderr and exit 0, so almost every input class is distinguished *only* by which
branch `x` selects. That makes `scanf` overflow/truncation semantics
behaviourally load-bearing rather than cosmetic.

## Mismatches found

### 1. `SIGPIPE` disposition — Rust exited 0 where C was killed by signal 13

**Status: found and fixed.**

The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs; a C
program inherits the default disposition. With stdout connected to a pipe whose
reader has already closed:

| input | C | Rust (before fix) |
|---|---|---|
| `1` | killed by signal 13, no output | exit code 0, no output |
| `0` | killed by signal 13, no output | exit code 0, no output |
| `""` | killed by signal 13, no output | exit code 0, no output |
| `abc` | killed by signal 13, no output | exit code 0, no output |

Reproduction (stdout = write end of a pipe with the read end closed):

```
C   : returncode=-13 stderr=b''
Rust: returncode=0   stderr=b''
```

**Cause:** `std::rt` calls `signal(SIGPIPE, SIG_IGN)` during Rust runtime
startup, so the failing `write` returned `EPIPE` (which the translation
discards with `let _ = ...`, matching C's ignoring of `printf`'s return value)
instead of raising the signal that terminates the C process.

Note that this is invisible to any test that compares only stdout: both
programs emit nothing on this path. It is only visible in the exit status —
exactly the failure mode Phase B warns about.

**Fix (in `src/main.rs`):** restore the default disposition as the first thing
`main` does, via a directly declared `extern "C" fn signal` (no new
dependency):

```rust
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

The C dies inside glibc's exit-time flush (its single small write is buffered),
whereas the Rust dies at the unbuffered `write` in `print_line`. Both are
killed by signal 13 having produced no output, so the three compared channels
agree.

Regression test: `broken_stdout_pipe_kills_both_with_sigpipe`. Verified
non-vacuous by commenting out the `restore_default_sigpipe()` call and
re-running — the test fails with `left: (None, Some(13))` vs
`right: (Some(0), None)`.

## Behaviours confirmed identical (no mismatch, but each was a real risk)

### `bad()` reads an uninitialized `char *` (CWE-457/824)

`bad()` calls `printLine(data)` with `data` never assigned. `printLine`'s
`line != NULL` guard therefore depends on a leftover stack slot. Instrumenting
a **copy** of the C source (in `$TMPDIR`, `c_src/` untouched) showed the slot is
consistently non-NULL and its first byte is 0:

```
PTR=0x7faa7af4f520 FIRST=0     # input ""
PTR=0x7f4c48406520 FIRST=0     # input "0"
PTR=0x7fec0f44a520 FIRST=0     # input "abc"
PTR=0x7f816d36e520 FIRST=0     # input "4294967296"
```

so `printf("%s\n", "")` emits exactly one byte, `0x0a`. The disassembly shows
`bad()` loading `-0x8(%rbp)`, a slot left behind by `main`'s `scanf` frame;
the value did not change when the environment size was perturbed by 5 KB. The
Rust models this as `Some("")`, which reproduces the observed `"\n"`. Pinned by
`bad_branch_prints_only_a_newline`, which asserts against the C's actual bytes.

### `scanf("%d")` overflow: saturate at `long`, then truncate to `int`

glibc accumulates in a `long`, saturates at `LONG_MIN`/`LONG_MAX` on overflow,
then stores the low 32 bits into the `int`. Confirmed against the C:

| input | C output | why |
|---|---|---|
| `2147483648` | `string` | truncates to `INT_MIN`, non-zero |
| `4294967296` | `\n` | low 32 bits are 0 → `bad()` despite a successful conversion |
| `4294967297` | `string` | low 32 bits are 1 |
| `99999999999999999999` | `string` | saturates to `LONG_MAX`, low 32 bits `0xffffffff` |
| `-99999999999999999999` | `\n` | saturates to `LONG_MIN`, low 32 bits `0` |
| 100 000 `9`s | `string` | unbounded digit run, still saturates |

Note the asymmetry: positive overflow reaches `good()`, negative overflow
reaches `bad()`. Saturating at `i32` instead of `i64`, or wrapping instead of
saturating, would flip these. Covered by `int_boundaries`,
`truncation_to_int_can_produce_zero`, `long_boundaries_and_saturation`,
`very_long_digit_runs`.

### `scanf` skips whitespace *across newlines*

`%d` consumes any run of `isspace` characters (`' '`, `\t`, `\n`, `\v`, `\f`,
`\r`) before the number, unlike `fgets`. `"\n\n\t 5"` reaches `good()`.
Covered by `scanf_skips_leading_whitespace_across_newlines`.

### Conversion failure leaves `x` at its initializer

`scanf` returning `EOF` (empty stdin, whitespace-only stdin) or `0` (matching
failure: `abc`, `.5`, `--5`, a lone `-` or `+`, `-  5`) does **not** write to
`x`, so `x` keeps the `0` from its declaration and `bad()` runs. The Rust models
this by only assigning when the emulated conversion succeeds. Covered by
`empty_input_is_input_failure`, `matching_failure_leaves_x_zero`,
`sign_then_eof_is_not_a_conversion`, `whitespace_only_input`.

### Only one item is read

`scanf` is called once, so trailing input is irrelevant: `0x10` converts `0`
and reaches `bad()`; `3.9` converts `3` and reaches `good()`. The Rust consumes
one byte past the digit run, which is unobservable because stdin is never read
again. Covered by `only_the_first_item_is_read`.

### stdout write errors are not reported

With stdout on `/dev/full` every write fails with `ENOSPC`; glibc discards the
failed exit-time flush and `main` has already returned `0`, so the C exits 0
silently. The Rust discards write errors the same way. Covered by
`stdout_write_failure_is_not_reported`.

### NUL and non-ASCII bytes

A leading `\0` or `\xff` is a matching failure, not whitespace, so `"\0\0 42"`
reaches `bad()` — the digits after the NULs are never reached. Covered by
`non_ascii_and_nul_bytes`.

## Mechanical sweeps

Beyond the hand-enumerated classes:

- `exhaustive_short_inputs_over_interesting_alphabet` — all 1-, 2- and 3-byte
  strings over `0129+- \t\nx.\0` (1 740 inputs), all three channels compared.
- `seeded_random_inputs` — 400 seeded xorshift64\* inputs up to 24 bytes,
  digit/sign/whitespace-biased with arbitrary bytes mixed in.

All agree on stdout, stderr and exit status.

## Final state

- Both programs build with no errors or warnings.
- `cargo test` in `translation/`: 21 passed, 0 failed, 0 ignored.
- `c_src/` sources unmodified (`c_src/src/main.c` md5
  `6fc45ddd08c75859efcc23e667743631`, `c_src/CMakeLists.txt` md5
  `02ba3005fed9b6d7d46c4fe335ac00d8`); the only addition under `c_src/` is the
  `build/` directory produced by the prescribed CMake invocation.
