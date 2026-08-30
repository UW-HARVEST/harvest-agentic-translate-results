# Differential verification: `c_src` vs `translation`

Record of every divergence class probed between the C reference and the Rust
translation, what the C actually does, and the outcome.

## How each program is run

```
# C reference (built once, per the task instructions)
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
# -> c_src/build/driver

# Rust
cd translation && cargo build --release
# -> translation/target/release/driver

# Differential suite (drives both as subprocesses)
cd translation && cargo test
```

Both are driven as subprocesses over stdin; stdout, stderr and exit status
(including termination by signal) are compared. Nothing is loaded as a library.

Compiler used for the reference: `gcc 11.5.0`, no `CMAKE_BUILD_TYPE`, so the C
is built unoptimized. GCC emits the expected
`warning: function returns address of local variable [-Wreturn-local-addr]`.

## Headline result

**No behavioral mismatch was found between the Rust translation and the C.**
All 25 tests pass, plus an independent shell sweep of 151 inputs with zero
mismatches. The defect list below is therefore not a list of bugs fixed in the
Rust; it is the list of divergence classes that were *checked*, each with the
evidence that the two agree. One real defect was found in the **test suite**
(§6) and fixed there.

## 1. `helperBad` returns the address of a local array (CWE-562)

```c
static char *helperBad()
{
    char charString[] = "helperBad string";
    return charString;      // dangling
}
```

This is undefined behavior, so there is no language-defined value to reproduce —
only what the compiled reference actually does. Verified empirically rather than
assumed:

- Disassembly of `helperBad` in the reference binary ends in
  `mov $0x0,%eax; ret` — GCC folds the dangling return to a **null pointer**.
- `printLine`'s `if (line != NULL)` therefore fails and the bad path prints
  **nothing at all — not even a newline**.
- Confirmed stable at `-O0`, `-O1`, `-O2`, `-O3` and `-Os`: input `0` produces
  zero bytes of stdout at every level.

The Rust models this as `helper_bad() -> None`, which reproduces the observed
bytes without unsafe code or a dangling reference. `harness_sanity` pins this
behavior literally so the whole suite cannot silently degrade if the bad path
ever starts emitting output.

Note: this is compiler-observed behavior, not a guarantee. A toolchain that
actually returned the stack address would print garbage or crash, and the
`None` model would then be wrong. Verified against the toolchain in use.

## 2. `scanf("%d")` truncates; it does not clamp to `int` range

The subtlest behavior in the program. glibc converts with `strtol` semantics
into a `long`, then stores through an `int *`. So out-of-range input
**saturates** at `LONG_MAX`/`LONG_MIN` and is then **truncated to the low 32
bits** — it does *not* clamp to `INT_MAX`/`INT_MIN`.

Truncation is observable because it decides which branch runs:

| input | `long` value | stored `int` | branch | stdout |
|---|---|---|---|---|
| `2147483648` | 2147483648 | `0x80000000` = INT_MIN | good | `helperGood1 string\n` |
| `4294967295` | 4294967295 | `-1` | good | `helperGood1 string\n` |
| `4294967296` | 4294967296 | **`0`** | **bad** | *(empty)* |
| `9223372036854775807` | LONG_MAX | `-1` | good | `helperGood1 string\n` |
| `-9223372036854775808` | LONG_MIN | **`0`** | **bad** | *(empty)* |
| `99999999999999999999999999` | saturates LONG_MAX | `-1` | good | `helperGood1 string\n` |
| `-99999999999999999999999999` | saturates LONG_MIN | **`0`** | **bad** | *(empty)* |

`4294967296` is the case worth highlighting: a plainly nonzero input takes the
*bad* branch. A translation that clamped to `INT_MAX` would print
`helperGood1 string` here and diverge. Confirmed by mutation: replacing
`value as i32` with `value.clamp(INT_MIN, INT_MAX)` fails
`int_range_edges`, `long_range_saturation` and `very_long_digit_strings`.

## 3. `scanf` reads across newlines

`%d` skips *all* leading whitespace, newlines included — the documented contrast
with `fgets`. So `"\n\n\n\n7"` converts to 7 and takes the good branch.
Covered for `' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'`, individually and
combined, including a 1200-byte leading whitespace run and a 64 KiB one.

## 4. Conversion failure leaves `x` at its initializer

```c
int x = 0;
scanf("%d", &x);   // return value ignored
```

On either input failure (EOF first) or matching failure (no digits), `scanf`
does not write to `x`, so `x` stays `0` and the **bad** branch runs. The return
value is ignored by the C, so a failed conversion is indistinguishable from an
explicit `0`. Both produce empty stdout and exit 0.

Covered: empty input, whitespace only, `/dev/null` stdin, `abc`, `-`, `+`,
`--1`, `.`, `.5`, `0x` , `- 1`, NUL byte, `0xff`, invalid UTF-8, a UTF-8 minus
sign, a fullwidth digit, and a UTF-8 BOM.

Only the *first* conversion happens; `%d` stops at the first byte that cannot
extend the number and the rest of stdin is never read (`0 1` → bad, `1 0` →
good, `0x10` → reads `0` → bad, `3.9` → reads `3` → good).

## 5. `SIGPIPE` disposition — a real C/Rust divergence, handled

A C program inherits `SIG_DFL` for `SIGPIPE` and is killed by it when stdout is
a pipe whose reader has closed (shell status **141**). The Rust runtime sets
`SIGPIPE` to `SIG_IGN` before `main`, which would turn the failed write into an
ignored error and **exit 0** instead.

`src/main.rs` undoes this in `restore_default_sigpipe()`. Measured:

```
c_src/build/driver              -> PIPESTATUS=141
translation/.../release/driver  -> PIPESTATUS=141
```

Confirmed the test is not vacuous: deleting the `restore_default_sigpipe()`
call makes `sigpipe_disposition_matches` fail (Rust exits 0, C dies on 13).
This is the one divergence class where the stock Rust runtime would have been
wrong by default, and it is invisible to any test that only feeds stdin and
reads stdout.

## 6. Defect found in the test suite: sign invisible without saturation

Found by mutation testing, not by a failing test. Mutating
`let negative = b == b'-'` to `let negative = true` — treating `+` as negative —
**initially survived the whole suite**.

Why it hid: for in-range values the sign cannot change the branch, because the
low 32 bits of `-v` are zero exactly when those of `v` are, and the program only
branches on `x != 0`. The sign becomes observable *only* under saturation, where
`+huge` → `LONG_MAX` → `-1` → good but `-huge` → `LONG_MIN` → `0` → bad. No
saturation test carried a `+` prefix, and the fuzz inputs were capped at 8 bytes
— too short to reach the `long` range at all.

Fixed by adding `sign_is_observable_under_saturation` (explicit `+`/`-` at and
past `LONG_MAX`/`LONG_MIN`) and `deterministic_fuzz_long_inputs` (10–35 byte
inputs over a digit-heavy alphabet, so overflow and signs interact). The mutant
is now caught. Verified against C: `+99999999999999999999999999` prints
`helperGood1 string`, `-99999999999999999999999999` prints nothing.

## 7. Output formatting

`printf("%s\n", line)` — the string then exactly one newline, no other spacing,
no trailing content. glibc rewrites this to `puts`, which emits the same bytes.
Neither program writes to stderr on any tested input; both exit `0`. Confirmed
non-vacuous: adding `eprintln!("note")` to the Rust fails the suite, and
dropping the `\n` or misspelling the string fails it too.

## Mutation testing summary

Passing differential tests only mean something if the tests can fail, so each
mutation below was injected into `src/main.rs`, built, and run against the
suite. All were caught; `src/main.rs` was restored byte-identical afterwards.

| mutation | caught by |
|---|---|
| `helper_bad` returns `Some("helperBad string")` (defect "fixed") | 19 tests incl. `harness_sanity` |
| `restore_default_sigpipe()` removed | `sigpipe_disposition_matches` |
| truncation replaced with clamp to `int` range | `int_range_edges`, `long_range_saturation` |
| saturation sign swapped (`LONG_MAX`/`LONG_MIN`) | `long_range_saturation`, `sign_is_observable_under_saturation` |
| `+` treated as negative | `sign_is_observable_under_saturation` *(added for this)* |
| `+` sign not accepted | `sign_handling`, `nonzero_values_take_good_branch` |
| branch `x >= 0` instead of `x != 0` | 20 tests |
| trailing `\n` dropped from `printLine` | 14 tests |
| `helperGood1 string` misspelled | 14 tests |
| only `' '` counts as whitespace | `leading_whitespace_is_skipped` |
| `'\n'` not counted as whitespace | `leading_whitespace_is_skipped` |
| `exit(1)` instead of `exit(0)` | 19 tests |
| stdout never flushed | 14 tests |
| extra line written to stderr | 14 tests |

One mutation was an equivalent mutant, not a survivor: `eprint!("")` writes zero
bytes and so is genuinely unobservable. Replacing it with a real stderr write
(`eprintln!("note")`) is caught, which confirms stderr is actually compared.

## Coverage against the C source

Every branch in the C is exercised:

- `printLine`: `line != NULL` (input `1`) and `line == NULL` (input `0`)
- `main`: `if (x)` taken and not taken
- `scanf`: conversion success, input failure (EOF), matching failure (no digits)
- `good()` / `bad()` / `helperGood1()` / `helperBad()`: all called

Also covered beyond the branch structure: `/dev/null` stdin, stdout redirected
to a regular file rather than a pipe, command-line arguments (ignored by the C's
`main()`), inputs larger than a stdio buffer (64 KiB), 10,000-digit numbers,
25 repeated runs for stability, all integers in `-300..=300`, and 1,000
deterministic fuzz cases.

## Status

- Both programs build without error.
- `cargo test` passes in `translation/` — 25 tests, in both debug and release.
- No test is `#[ignore]`d, skipped or conditionally disabled. Every assertion is
  differential (expected value = whatever the C did), except `harness_sanity`,
  which deliberately pins C's observed behavior so the harness cannot silently
  compare a program against itself.
- Nothing in `c_src/` was modified; both source files retain their original
  checkout mtime. The only addition is the generated `c_src/build/` directory,
  created by the build command in the task instructions.
