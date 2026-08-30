# Differential testing report

Ground truth: `c_src/src/main.c`, built with CMake (`cmake -S c_src -B <dir> && cmake --build <dir>`).

```c
int main() {
    int x = 1, y = 1;
    scanf("%d %d", &x, &y);
    div_t result = div(x, y);
    printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
    return 0;
}
```

Both programs are compared by execution only: stdout, stderr and exit status
(including death-by-signal) for identical stdin. The Rust code is never loaded
as a library.

- C binary: `<target>/c_reference_build/driver` (built by the test harness, out of tree)
- Rust binary: `translation/target/release/driver`

## Enumerated input classes

The C program is tiny but branches in more places than it looks:

| Class | Reached by | Result |
|---|---|---|
| 0 conversions (input failure at EOF) | `""`, `"   "`, `"\n\n\n"` | `x`,`y` keep initialiser `1` → `div(1,1)` |
| 0 conversions (matching failure) | `"abc"`, `"-"`, `"+"`, `"- 7 2"`, `"--7"`, `".5"` | same as above |
| 1 conversion | `"7"`, `"7 abc"`, `"7 -"`, `"0x10 2"`, `"7.5 2"`, `"1e3 2"` | `y` keeps `1` → `div(x,1)` |
| 2 conversions | `"7 2"` and all four sign combinations | truncating division, remainder takes numerator's sign |
| whitespace skipping | `"\t 7 \t\n\n  2  "`, `"7\r\n2\r\n"`, `"\v7\f2"` | `scanf` reads across newlines (unlike `fgets`) |
| `long` → `int` truncation | `"4294967296"`, `"2147483648"`, `"4294967295"` | wraps: `0`, `INT_MIN`, `-1` |
| strtol saturation then truncation | `"99999999999999999999999"`, `"9223372036854775808"` | saturates to `LONG_MAX` → truncates to `-1` |
| `div(x, 0)` — UB | `"5 0"`, `"0 0"`, `"5 4294967296"` | `idiv` faults → killed by **SIGFPE (8)**, no stdout |
| `div(INT_MIN, -1)` — UB | `"-2147483648 -1"`, `"2147483648 -1"` | quotient overflow → killed by **SIGFPE (8)** |
| broken stdout, SIGPIPE = `SIG_DFL` | stdout pipe with no reader | killed by **SIGPIPE (13)** |
| broken stdout, SIGPIPE = `SIG_IGN` | as above, disposition inherited | `fflush` fails, **exit 0**, empty stderr |
| stdin without EOF, token terminated | `"7 2 "` on an open pipe | prints and exits — does not wait for EOF |
| stdin without EOF, token unterminated | `"7 2"` on an open pipe | **C itself blocks** waiting for one more byte |

Coverage is backed by 20 test functions in `tests/differential.rs`, including two
deterministic randomised sweeps (400 random byte strings over the alphabet the
parser branches on, and 250 random integer pairs biased toward boundary values;
verified non-degenerate: 388 distinct inputs, 28 of the pairs hit a faulting
divisor).

## Mismatches found and fixed

### 1. `print!` panicked instead of failing silently like `printf`

**Symptom.** With stdout wired to a pipe that has no reader, and SIGPIPE
inherited as `SIG_IGN`:

```
C:    exit=0, stdout empty, stderr empty
Rust: killed by SIGABRT (6), stderr:
      thread 'main' panicked at library/std/src/io/stdio.rs:1166:9:
      failed printing to stdout: Broken pipe (os error 32)
```

**Cause.** The translation used `print!`, whose expansion panics if the write
fails. In C, `printf`/`fflush` merely return a negative value; `main` still
returns 0. Because `Cargo.toml` sets `panic = "abort"`, the panic became a
SIGABRT — wrong exit status *and* extra bytes on stderr.

**Fix.** Format into a `String` and write it with `write_all`, discarding the
`io::Result` (`let _ = ...`) exactly as C discards `printf`'s return value.
See `broken_stdout_with_ignored_sigpipe`.

### 2. The Rust runtime forced `SIGPIPE` to `SIG_IGN` before `main`

**Symptom.** Same setup but with SIGPIPE inherited as `SIG_DFL`:

```
C:    killed by SIGPIPE (13)
Rust: exit 0   (after fix #1 removed the panic)
```

**Cause.** `std::rt::init` installs `SIGPIPE = SIG_IGN` before `main` runs, so a
failing write returns `EPIPE` instead of killing the process. The C program never
touches SIGPIPE and simply runs with whatever it inherited, so both dispositions
must be honoured. Merely hard-coding `signal(SIGPIPE, SIG_DFL)` in `main` would
have been wrong too — it would break the inherited-`SIG_IGN` case above.

**Fix.** Register a constructor in `.init_array`, which runs during process
start-up *before* the Rust runtime initialises. It reads the inherited
disposition and stores it in an `AtomicUsize`; `main` then restores it as its
first action. Both dispositions now behave as in C. See
`broken_stdout_with_default_sigpipe`.

> This one was easy to miss because the surrounding shell/test harness had
> SIGPIPE set to `SIG_IGN`, which is why the C program quietly exited 0 rather
> than dying — the behaviour to replicate was itself environment-dependent.

### 3. `read_to_end` on stdin blocked where C did not

**Symptom.** Input `"7 2 "` written to a pipe that is left open (no EOF):

```
C:    prints "quotient: 3, remainder: 1\n" and exits
Rust: hangs forever
```

**Cause.** The translation slurped all of stdin with `read_to_end` before
parsing. `scanf` instead pulls from a buffered `FILE*` and stops as soon as the
second `%d` is terminated, so it never waits for EOF.

**Fix.** Replaced the eager slurp with an `Input` struct that fills a 4096-byte
buffer (glibc's `BUFSIZ`) lazily via `peek`/`bump`, so exactly as much input is
demanded as `scanf` demands. This also faithfully preserves the converse: for
`"7 2"` with no terminating byte, the C program blocks waiting for one more
character, and so does the translation. See
`does_not_wait_for_eof_when_input_is_terminated` and `blocks_exactly_where_c_blocks`.

## Behaviours verified as already correct

These were the highest-risk areas; they were probed explicitly and matched
without changes:

- **`div` faults, not panics.** `div(x, 0)` and `div(INT_MIN, -1)` are UB in C;
  on x86-64 `idiv` raises SIGFPE. The translation issues `cdq; idiv` via inline
  asm, so it dies by signal 8 with empty stdout, exactly like the C binary. A
  `checked_div` + Rust panic would have produced SIGABRT and a stderr message.
- **`%d` goes through a `long`.** glibc converts into a `long`, saturating at
  `LONG_MAX`/`LONG_MIN`, and only then truncates into the `int`. Hence
  `"99999999999999999999999"` → `-1` and `"4294967296"` → `0`. Reproduced with
  `i64` accumulation plus `as u64 as u32 as i32` narrowing.
- **Untouched variables keep `1`.** On matching or input failure `scanf` does not
  write through the pointer, so the initialisers survive; the nested
  `if let Some` structure mirrors this, including the case where `x` converts but
  `y` does not.
- **Sign handling.** A lone `"-"`/`"+"`, or a sign followed by a non-digit
  (`"- 7"`, `"--7"`), is a matching failure — not a zero.
- **Whitespace set.** The C-locale `isspace` bytes (space, `\t`, `\n`, `\v`,
  `\f`, `\r`) are all skipped, including between the two conversions.
- **Non-UTF-8 stdin.** NUL bytes, `0x80`–`0xff` and full-width digits are handled
  bytewise; the translation parses `&[u8]`, never `str`, so no decoding error
  can occur.
- **Very long tokens.** 5000-digit numbers and 10000-byte whitespace runs (which
  span multiple 4096-byte buffer refills) behave identically.

## Final state

- `c_src/` unmodified — sources untouched; the test harness configures CMake
  into `target/c_reference_build/` rather than writing into the C tree.
- `cargo build --release`: clean, no warnings.
- `cargo test` (debug and release): 20 tests, 20 passed, 0 ignored, 0 skipped.
- Independent byte-exact sweep outside the test suite: 465 inputs, 0 mismatches.
