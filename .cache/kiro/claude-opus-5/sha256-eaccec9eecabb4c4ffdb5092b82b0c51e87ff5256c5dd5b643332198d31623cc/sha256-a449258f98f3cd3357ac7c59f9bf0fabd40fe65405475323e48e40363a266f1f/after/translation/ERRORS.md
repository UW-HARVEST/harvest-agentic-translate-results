# Verification record: `c_src/src/main.c` → `translation/src/main.rs`

Ground truth is the C program. Everything below was established by running both
executables and diffing stdout, stderr and exit status.

## Headline result

**No behavioral mismatch was found between the two programs.** Across 1740
differential cases in `tests/differential.rs`, plus a further ~15,000 ad-hoc and
randomized cases run during investigation, stdout, stderr and exit status were
byte-identical on every input.

One real defect *was* found and fixed — in the test suite, not in the
translation. It is recorded in "Defect 1" below, because a gap that lets a wrong
translation pass is the same class of problem as a wrong translation.

## Build and run commands

| | command | binary |
|---|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` | `translation/target/release/driver` |

Both build clean, no errors and no warnings. `cargo test` builds the C binary
itself (guarded by a `OnceLock` so parallel test threads invoke CMake once), so
the suite is self-contained.

## Branches enumerated from the C source

`main`:

- **M0** `scanf` completes 0 conversions → `x=0`, `y=123`, `z=0`
- **M1** completes 1 → `x` set, `y` stays at its initialiser 123, `z=0`
- **M2** completes 2 → `x`, `y` set, `z=0`
- **M3** completes 3
- **M4** `printf("Result: %d\n", result)` then `return 0`, on every path

`multi_stage(int x, int z)` — note `y` is the file-scope global read directly,
not a parameter:

- **S1** `x != 1` → `Error: x != 1`, result 1, `goto fail`
- **S2** `x == 1 && y != 2` → `Error: x == 1 but y != 2`, result 2, `goto fail`
- **S3** `x == 1 && y == 2 && z != 3` → `Error: x == 1 and y == 2, but z != 3`, result 3, `goto fail`
- **S4** all three match → `Ok!` and `return` **before** the `fail:` label, so
  `Operation failed` is not printed. This is the only path that skips it.
- **S5** the `fail:` label → `Operation failed`

Each branch has a named test. `full_truth_table_across_separators` additionally
crosses x ∈ {1,2,0,-1} × y ∈ {2,3,0,-2} × z ∈ {3,4,0,-3} × 5 separator styles so
no ordering of the three checks can hide behind another.

## Defect 1 — test suite could not distinguish 32-bit from 16-bit truncation

**Severity: real. Found by mutation testing, now fixed.**

`scanf("%d", …)` stores through an `int *`, so the parsed value is truncated to
exactly 32 bits. The original suite covered truncation using values of the form
`k·2³² + 1`. Those values have the same low 16 bits *and* low 32 bits, so they
cannot tell the widths apart.

Demonstrated by mutating `Some(acc as i32)` to `Some(acc as i16 as i32)`: the
mutant passed the entire suite. It is genuinely wrong — for input `65537 2 3`
(`0x00010001`) the C prints `Error: x != 1` while the i16 mutant prints `Ok!`,
because `65537 as i16 == 1`.

Fix: added `truncation_width_is_exactly_32_bits`, which sweeps values whose low
8 bits and low 16 bits alias 1/2/3 without the `int` value doing so
(`k·256 + {1,2,3}`, `k·65536 + {1,2,3}`), their negative counterparts
(`-65535`, `-255`, …), and the sign-extension boundaries 127/128/255/256/
32767/32768/65535/65536 in each of the three token positions. The i8, i16 and
"no truncation" mutants are all killed now.

## Behaviors that were verified rather than assumed

These are the places a plausible translation would have diverged. Each was
checked against the compiled C program, not against the C standard.

### `%d` overflow saturates, it does not wrap

glibc's `%d` accumulates the digit run and converts it with `strtol`
semantics, which **saturates** at `LONG_MAX`/`LONG_MIN` on overflow; the
saturated `long` is then truncated on the store to `int`. This is observably
different from 64-bit wrapping arithmetic:

| input | C output | wrapping would give |
|---|---|---|
| `1 4294967298 3` (2³²+2, fits in `long`) | `Ok!` — truncates to y=2 | same |
| `1 18446744073709551618 3` (2⁶⁴+2) | `Error: x == 1 but y != 2` — saturates to `LONG_MAX`, y = −1 | y = 2 → `Ok!` |
| `1 -18446744073709551614 3` (−(2⁶⁴−2)) | y error — saturates to `LONG_MIN`, y = 0 | y = 2 → `Ok!` |

`translation/src/main.rs` implements exactly this (saturating `i64`
accumulation, then `as i32`), and matches. Covered by
`overflow_saturation_and_truncation` and `low_32_bits_sweep`.

Note this behavior depends on `long` being 64-bit. On an ILP32 target the C
program itself would behave differently; the translation matches the LP64 build
that `c_src/CMakeLists.txt` produces here.

### `%d` crosses newlines

`%d` skips arbitrary leading whitespace including `\n`, so `1\n2\n3\n`,
`1\n\n\n2\n\n\n3\n` and 10,000 interleaved newlines all reach the success path.
A `fgets`-style line-at-a-time reader would have failed these. Verified by
mutating the whitespace predicate to exclude `\n` — killed.

The whitespace set is C's `isspace` in the "C" locale exactly: space, `\t`,
`\n`, `\v` (0x0b), `\f` (0x0c), `\r`. Bytes 0x1c–0x1f are **not** whitespace and
terminate a token; `non_ascii_and_control_bytes` pins this down.

### `strtol` base-10 token termination

`0x10` parses as `0` and stops at `x`; `1e5` parses as `1`; `1.5` parses as `1`;
`1_2` parses as `1`; `010` parses as ten, not eight. All covered by
`strtol_token_termination`.

### A sign with no digits is a matching failure, not zero

`-`, `+`, `- 1`, `--1`, `+-1` leave the target variable untouched rather than
setting it to 0. See "Equivalent mutants" below for why this turns out to be
unobservable in *this* program even though the translation implements it
correctly.

### Arbitrarily long digit runs

A 100,001-digit token is accepted and converted (leading zeros do not count
toward overflow). `very_long_digit_runs` covers 5,001-digit and 100,001-digit
tokens; both programs agree.

### Exit status is always 0, stderr is always empty

The C `main` ends in `return 0` unconditionally, on every error path. Every
error message goes to **stdout**, not stderr. A translation that mapped result
1/2/3 onto the process exit code would pass a stdout-only comparison and fail
here — verified by mutating the Rust to `exit(result)`, which is killed.

### I/O failure paths

- stdout redirected to `/dev/full` (writes fail with ENOSPC): the C ignores every
  `printf` return value and still exits 0. The Rust `let _ = write!(…)` matches.
  Mutating those to `.unwrap()` is killed.
- stdin closed, stdin `/dev/null`, stdin a directory (`read` → EISDIR): all
  produce the M0/S1 path identically in both.
- Up to 1 MiB of unread trailing stdin: both programs exit without draining it,
  and the test harness tolerates the resulting `EPIPE` on its writer thread.

### argv is ignored

`main()` takes no parameters. `argv_is_ignored` confirms extra arguments change
nothing in either program.

## Equivalent mutants — unobservable, not untested

Two deliberate defects survived the suite. Both were investigated and shown to
be **behaviorally equivalent to the C program**, so no test could have killed
them:

1. a lone sign yielding `Some(0)` instead of a matching failure
2. `scanf` continuing past a failed conversion instead of stopping at the first

Evidence: each mutant was built and compared against the C binary over 13,313
inputs — an exhaustive enumeration of all strings of length ≤ 4 over the
class-covering alphabet `{1,2,3,-,+,space,x,0,\n}`, plus targeted lone-sign
inputs in all three token positions, plus 5,500 randomized cases. Zero
distinguishing inputs.

The reason is structural: a failed conversion can only change a variable that
either already fails its own check, or is never reached. `x != 1` and `y != 2`
each return before `z` is examined, and if `x` and `y` both converted
successfully then a failure can only occur at `z`, where there is nothing left
to continue to. Likewise `x`'s and `z`'s initialisers are both 0, so
"untouched" and "set to 0" are indistinguishable, and `y`'s initialiser 123 and
a mutant's 0 are both `!= 2`.

The translation implements the correct (C-faithful) behavior in both cases
anyway; this note exists so a future reader does not mistake the survival for a
coverage hole.

## Also unobservable: `y`'s initialiser value

`static int y = 123;` is only ever compared against 2, and the only input class
that leaves it at 123 (M1) reports `y != 2` regardless. Changing 123 to any
value other than 2 produces identical output on all inputs. Changing it *to* 2
is observable and is killed by `m1_one_conversion` (input `1` → `Ok!` instead of
the y error).

## Known divergence outside the compared surface: SIGPIPE

A Rust program starts with `SIGPIPE` set to `SIG_IGN`; a C program does not. If
stdout were a pipe whose reader had already exited, the C program would die with
signal 13 where the Rust program would exit 0.

This is **not reachable for this program**: its entire output is at most 60
bytes, which always fits in a pipe buffer, so `write` never returns `EPIPE`
before the process exits. Tested with `… | head -c 1`: both exit 0. It is
recorded here for completeness rather than fixed, since suppressing it would
require an `unsafe` `signal(2)` call for a path no input can reach.

## Mutation testing summary

The suite was validated by injecting 38 deliberate defects into
`translation/src/main.rs`. **36 were killed**; the 2 survivors are the
equivalent mutants documented above. Categories exercised:

- numeric conversion: saturating vs wrapping overflow, `i32` vs `i64`
  accumulator, truncation width (i8 / i16 / i32 / none), minus sign ignored,
  plus sign rejected, lone sign treated as zero
- scanning: three variants of the `isspace` set (dropping `\v`/`\f`, only
  space and tab, wrongly adding 0x1c–0x1f), refusing to cross newlines, and
  removing the one-byte pushback
- `scanf` wiring: `y`/`z` assignment order swapped, continuing past a failed
  conversion, dropping the third conversion
- initialisers: `y` starting at 2, `x` starting at 1, `z` starting at 3
- control flow: no early `return` on the success path, `z` checked before `y`,
  and `!=` weakened to `<` or `>` in each of the three comparisons
- result codes: 1↔3 swapped, 2↔3 swapped, all forced to 0
- output formatting: all four message strings reworded, `Ok!` and `Result:`
  trailing newlines removed, `Result:` spacing changed, the fail message omitted
- process behavior: `exit(result)` instead of `return 0`, `printf` errors
  propagated as panics, error messages sent to stderr instead of stdout

`translation/src/main.rs` was restored byte-for-byte after every mutation and
verified identical to the original.

## Completion gate

- [x] both programs build with no errors
- [x] 1740 enumerated differential cases produce identical stdout, stderr and exit status
- [x] `cargo test` passes in `translation/` (24 tests, debug and release)
- [x] no test is disabled, skipped or `#[ignore]`d; the only conditional path is
      the `/dev/full` check, which `panic!`s rather than returns on Linux
- [x] nothing in `c_src/` modified — `src/main.c` and `CMakeLists.txt` retain
      their original mtimes; only `c_src/build/` was added, as the build
      instructions require
