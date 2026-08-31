# Verification log — C vs Rust differential testing

`c_src/src/main.c` reduces to:

```c
void driver(int x, int y) { int result = x | ~y; printf("%d", result); puts(""); }
int main() { int x = 0, y = 0; scanf("%d", &x); scanf("%d", &y); driver(x, y); return 0; }
```

The source is written with ISO 646 alternative tokens (`%:include` = `#include`,
`<%`/`%>` = `{`/`}`) and `<iso646.h>` macros (`bitor` = `|`, `compl` = `~`).
That is cosmetic; it does not change semantics.

There is no explicit `if` in the C program. All of its branching lives inside
the two `%d` conversions and in the fact that the return value of `scanf` is
**ignored**, so a failed conversion silently leaves the variable at its
initializer of `0`.

## How this was verified

`translation/tests/differential.rs` spawns both binaries as subprocesses
(never as a library), pipes identical bytes to stdin, and compares stdout,
stderr and exit status. The C binary is built by the test via CMake on first
use if `c_src/build/driver` is absent. 25 tests, ~1,100 distinct inputs
including a 400-case deterministic fuzz walk. Nothing is `#[ignore]`d.

## Mismatches found

### 1. Synthetic NUL byte pushed back after a bare sign at EOF

**Status:** fixed in `src/main.rs`.

The original translation handled a sign followed by end-of-input via a helper:

```rust
fn negative_advance(stream: &mut CStdin, c: &mut u8) {
    *c = stream.getc().unwrap_or(0);   // EOF becomes a 0 byte
}
```

The `0` sentinel then fell through to the non-digit branch and was
`ungetc`'d, injecting a NUL byte that was never in the input into the stream
for the *next* conversion to read.

glibc does not do this. In `vfscanf`, `ungetc` is a macro that short-circuits
on `EOF`:

```c
#define ungetc(c, s) ((void) (c == EOF || (--read_in, _IO_sputbackc (s, c))))
```

so after `scanf("%d")` consumes a sign and then hits EOF, nothing is pushed
back and the stream stays at EOF.

The fix returns `None` directly on EOF-after-sign instead of manufacturing a
byte, and folds the sign handling inline so the "one offending character is
pushed back, the sign is not" rule is explicit:

```rust
if c == b'-' || c == b'+' {
    negative = c == b'-';
    match stream.getc() {
        Some(b) => c = b,
        None => return None,   // glibc's ungetc is a no-op for EOF
    }
}
if !c.is_ascii_digit() {
    stream.ungetc(c);          // only this character goes back
    return None;
}
```

**Observability:** with exactly two conversions and no further reads, the
divergence is not reachable from stdout — the injected NUL would itself be a
matching failure for the second `%d`, leaving `y = 0`, which is what EOF
produces too. Inputs `"-"`, `"+"`, `"-\n"`, `"5 -"` and `"--"` agree before
and after the change. It was fixed because the modeled stream semantics were
wrong, not because a test caught it; a third `scanf` would have exposed it.

No other mismatch was found. Every other input class matched on the first run.

## Behaviors deliberately preserved (not "fixed")

- **`scanf` crosses newlines.** `%d` skips all `isspace` characters, newlines
  included, so `"5\n\n\n3"` and `"5 3"` are identical inputs. Covered by
  `scanf_reads_across_newlines`.
- **Failed conversions leave `0`.** `scanf`'s return value is never checked,
  so `"abc"` yields `x = y = 0` and prints `0 | ~0 == -1`. Empty input prints
  `-1` and exits `0` — no error path, no stderr, no non-zero status. The C
  program has **no** input that produces stderr output or a non-zero exit
  status; the tests assert that rather than assuming it.
- **Only one character is pushed back on a matching failure.** glibc consumes
  the sign into its work buffer and does not restore it, so `"--5"` gives
  `x = 0` (failure) and `y = -5`, printing `0 | ~(-5) == 4`. Covered by
  `bare_and_doubled_signs`.
- **`long` saturation, then truncation to `int`.** glibc converts `%d` with
  `strtol`, which clamps to `LONG_MAX`/`LONG_MIN` on overflow, and the result
  is then stored into an `int`, truncating. So `"9223372036854775808"` becomes
  `LONG_MAX` and truncates to `-1`, while `"-9223372036854775808"` truncates
  to `0`. The Rust accumulator saturates at `i64::MIN`/`i64::MAX` and casts
  with `as i32` to match. Verified digit-by-digit for 1–40 digits of both
  signs, and against 100,000-digit inputs.
- **Base 10 only.** `"0x10"` parses as `0` with `'x'` pushed back; `"008"` is
  8, not an invalid octal.
- **C-locale `isspace`.** `0xa0` (Latin-1 non-breaking space) is not
  whitespace and not a digit, so it is a matching failure. The program never
  calls `setlocale`, so this holds regardless of the environment.
- **Output shape.** `printf("%d", result)` then `puts("")` — the decimal value
  and exactly one `\n`, no padding, no trailing space.

## Test-suite validity check

To confirm the tests are discriminating rather than vacuous, six mutations
were injected into the Rust source and the suite re-run; every one was caught,
and the restored source passes:

| Mutation | Result |
|---|---|
| `x \| !y` → `x & !y` | 25 / 25 tests fail |
| drop `long` saturation (wrap instead) | 3 tests fail |
| drop the `puts("")` newline | 25 / 25 tests fail |
| stop skipping `\n` while seeking a number (`fgets`-like) | 3 tests fail |
| `exit(1)` instead of `return 0` | 24 tests fail |
| drop the `ungetc` pushback on matching failure | 4 tests fail |

## Known environment-level difference (not input-driven)

If stdout is a pipe whose reader has closed, the C program takes `SIGPIPE`
and dies from the signal (shell status 141) while Rust masks `SIGPIPE` at
startup and would exit `0`. This is not reachable through any *input*, and it
does not occur when stdout is a file or a pipe that is fully read — both were
checked, including `>&-` (closed stdout), where both programs exit `0` with no
stderr. Correcting it would require `signal(SIGPIPE, SIG_DFL)` via a `libc`
dependency, which was not added rather than introducing a new dependency for a
case outside the comparison.

## Status

- Both programs build with no errors (`cmake --build .`, `cargo build --release`).
- `cargo test` passes in both dev and release profiles: 25 passed, 0 failed,
  0 ignored.
- `c_src/` is unmodified apart from the untracked `c_src/build/` output
  directory created by CMake.
