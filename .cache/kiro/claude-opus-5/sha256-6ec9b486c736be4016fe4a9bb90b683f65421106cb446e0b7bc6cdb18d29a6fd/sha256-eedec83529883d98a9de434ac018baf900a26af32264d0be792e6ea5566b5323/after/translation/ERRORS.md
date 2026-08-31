# Differential verification log

Comparison of `c_src` (ground truth) against the Rust crate in `translation/`,
by running both executables as subprocesses and diffing stdout, stderr and exit
status.

## What the C program does

`c_src/src/main.c` is 3 functions:

```c
static void print_hex(unsigned char *p, int len);  // "%02x" per byte, then "\n"
void driver(int x) { print_hex((unsigned char *)&x, sizeof(x)); }
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

The return value of `scanf` is discarded, so a failed conversion is
indistinguishable from reading a literal `0`: `x` simply keeps its initializer.
Every input therefore produces exactly 9 bytes on stdout, nothing on stderr, and
exit code 0. All of the observable variation comes from `scanf("%d")`.

## Input classes enumerated from the C source

| Path in the C / glibc | Example input | stdout |
| --- | --- | --- |
| input failure: EOF while skipping whitespace | `""`, `" \t\n\v\f\r"` | `00000000` |
| matching failure: first non-whitespace byte is not a digit or sign | `abc`, `.5`, `/`, `:`, `\x00`, `\xff` | `00000000` |
| matching failure: sign then EOF | `-`, `+` | `00000000` |
| matching failure: sign then non-digit | `-  5`, `--5`, `+a` | `00000000` |
| successful conversion | `42` | `2a000000` |
| conversion stops at first non-digit (pushed back) | `42abc`, `3.9`, `7 9` | `2a000000`, `03000000`, `07000000` |
| leading whitespace skipped across newlines | `"\n\n\n7"` | `07000000` |
| leading zeros, not treated as octal | `010` | `0a000000` |
| `long` -> `int` truncation | `2147483648` | `00000080` |
| `strtol` saturation at `LONG_MAX`, then truncation | `99999999999999999999` | `ffffffff` |
| `strtol` saturation at `LONG_MIN`, then truncation | `-99999999999999999999` | `00000000` |
| `print_hex` byte order (all 4 indices distinct) | `67305985` | `01020304` |

All of these are asserted in `tests/differential.rs`, together with an
exhaustive sweep of `-600..=600`, neighbourhoods of every conversion limit,
powers of two up to 2^39, and ~1800 deterministically seeded fuzz cases.

## Mismatches found

### 1. Broken stdout pipe: Rust exited 0 where C is killed by SIGPIPE

**Severity:** real behavioural divergence in exit status.

**Symptom.** With stdout connected to a pipe whose read end is already closed:

```
C: returncode=-13 (killed by signal 13: SIGPIPE)   # 141 as a shell reports it
R: returncode=0
```

**Cause.** Two things combined:

1. The Rust standard library installs `SIG_IGN` for `SIGPIPE` before `main`
   runs. A C program inherits the default disposition, so its `write` is
   interrupted by a fatal signal; with the signal ignored, Rust's `write`
   instead returns `EPIPE`.
2. `print_hex` discarded the result of the write (`let _ = out.write_all(..)`),
   so the `EPIPE` was swallowed and the program fell through to a normal
   `return 0`.

**Fix.** Restore the default `SIGPIPE` disposition at the top of `main`, by
declaring `signal` via `extern "C"` (libc is already linked into every
`*-unknown-linux-gnu` Rust binary, so this needs no new dependency):

```rust
extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
unsafe { signal(SIGPIPE, SIG_DFL); }
```

The write error is still ignored, which is correct: C's `printf` return value is
also unchecked, and that is what makes both programs exit 0 when stdout is
`/dev/full` (an `ENOSPC` failure, not a signal). Both behaviours are pinned by
`broken_stdout_pipe_kills_both_the_same_way` and
`full_stdout_device_is_handled_the_same`.

**Regression check.** Commenting out the `restore_default_sigpipe()` call makes
`broken_stdout_pipe_kills_both_the_same_way` fail with
`C Err(13) vs Rust Ok(0)`, so the test genuinely covers the fix rather than
passing by accident.

## Behaviours checked and found already correct

These were the likely failure points; each was probed against the C binary and
already matched, so they are recorded as verified rather than fixed.

- **`strtol` saturation, not wrapping.** glibc's `%d` converts through
  `strtol`, clamping the magnitude at `LONG_MAX` / `LONG_MIN`, and only then
  truncates to `int`. So `99999999999999999999` yields `ffffffff` (`-1`) and
  `-99999999999999999999` yields `00000000` (`0`) — not a modular reduction of
  the literal. The Rust `Scanner` reproduces this with an explicit saturating
  accumulator. Confirmed stable for digit runs of 20, 64, 100, 1000 and 5000.
- **Leading zeros do not saturate.** 5000 leading zeros followed by `7` still
  gives `07000000`, because zeros add no magnitude.
- **`scanf` crosses newlines** while skipping leading whitespace, unlike
  `fgets`. `"\n\n\n7"` reads `7`.
- **Whitespace set** is exactly `isspace`: space, `\t`, `\n`, `\v`, `\f`, `\r`.
- **Failed conversion leaves `x` untouched**, so `x` stays at its `= 0`
  initializer rather than being overwritten with a partial result.
- **Byte order** is the platform's; `to_ne_bytes` matches reinterpreting `&x` as
  `unsigned char *`. Verified with `67305985` -> `01020304`.
- **`%02x` on an `unsigned char`** never needs truncation or a wider field, and
  is lowercase.
- **Exactly one trailing newline**, total output length 9 bytes.
- **stderr is always empty** and the exit code is always 0 on a normal run,
  including on every error path, because `scanf`'s return value is unused.
- **argv is ignored**, since `main` is declared with no parameters.

## Final state

- `c_src/` unmodified (only the ignored `c_src/build/` output tree was created).
- `cargo build --release` and `cargo test` both clean.
- 33 tests pass in both the debug and release profiles; none is `#[ignore]`d,
  skipped or disabled.
