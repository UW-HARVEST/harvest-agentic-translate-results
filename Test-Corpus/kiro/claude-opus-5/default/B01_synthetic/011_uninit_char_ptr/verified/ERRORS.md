# Differential verification of the C → Rust translation

The C program in `../c_src/src/main.c` is the ground truth. `translation/` must
produce byte-identical stdout, byte-identical stderr and an identical exit
status for the same stdin.

Reference build (the one the comparison uses):

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
# runs as: ./c_src/build/driver
```

```
cd translation && cargo build --release
# runs as: ./translation/target/release/driver
```

`CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no `CMAKE_C_FLAGS`, so the C is
compiled with **no optimization flags**. That detail matters; see mismatch 2.

## What the program does

```c
void printLine(const char *line) { if (line != NULL) printf("%s\n", line); }
void bad()  { char *data;                  printLine(data); }  /* CWE-457 */
void good() { char *data; data = "string"; printLine(data); }
int  main() { int x = 0; scanf("%d", &x); if (x) good(); else bad(); return 0; }
```

Every run exits 0 and writes nothing to stderr. The only observable variation is
stdout: `"string\n"` when the scanned value is non-zero, `"\n"` when it is zero
(including when `scanf` converts nothing and `x` keeps its initial `0`).

## Mismatches found

### 1. Exit status under a closed stdout — SIGPIPE was ignored (fixed)

**Symptom.** With stdout on a closed pipe, the two programs disagreed on exit
status while agreeing on stdout and stderr:

```
$ echo 1 | ./c_src/build/driver | true          ; # C
141        (128 + SIGPIPE)
$ echo 1 | ./translation/target/release/driver | true
0
```

**Cause.** A C program inherits `SIGPIPE` at `SIG_DFL`, so the failing `printf`
write kills the process. The Rust runtime installs `SIG_IGN` for `SIGPIPE`
before `main` runs, which turns the same write into an `EPIPE` error; the
translation discarded that error and fell through to `return 0`.

**Fix.** `restore_default_sigpipe()` in `src/main.rs` resets the disposition to
`SIG_DFL` as the first statement of `main`, restoring C's behavior. Covered by
`sigpipe_kills_both_programs_alike`, which asserts both binaries die from signal
13 on both the `good()` and the `bad()` branch.

Note this is the one place the check has to be more than "compare stdout": a
stdout-only assertion passes here while the exit statuses differ by 141.

### 2. `bad()` reads an uninitialized pointer — output is build-dependent (verified, not a defect in the translation)

`bad()` reads `data` before assigning it, which is undefined behavior, so there
is no value the translation can derive from the source alone. The behavior of
the **reference build** was measured instead:

| C build | `bad()` stdout |
| --- | --- |
| no `-O` flag (what `CMakeLists.txt` produces) | `"\n"` |
| `-O1`, `-O2`, `-O3`, `-Os` | *empty* |

At `-O0` the stale stack slot left by `scanf` holds a non-NULL pointer to a NUL
byte, so `printLine`'s `line != NULL` guard passes and `printf("%s\n", …)`
emits only the newline. From `-O1` up, GCC 11.5.0 exploits the UB and the call
produces no output at all.

The translation models the `-O0` result — `bad()` calls `print_line` with
`Some("")`, i.e. a non-NULL pointer to an empty string — because that is what
the prescribed build command produces. **If the C is ever rebuilt with
optimization enabled, `bad()` must become a no-op (`print_line(out, None)`) for
the outputs to keep matching.**

The `-O0` value was checked for stability before relying on it: 200 consecutive
runs, `PAD` environment padding of 0/1/10/100/1000/5000 bytes, `env -i` with an
empty environment, argv padding of 0/1/5/50/500 bytes, and a different working
directory all produced exactly `"\n"`.

Consequence: `printLine`'s `line == NULL` branch is unreachable in the reference
binary. No input can exercise it, so no test asserts on it.

### 3. Integer range handling — no mismatch, but the non-obvious cases are pinned

No divergence was observed here; the cases are recorded because they look like
bugs and a naive translation gets them wrong.

glibc's `%d` converts with `strtol`, which **saturates** at `LONG_MAX` /
`LONG_MIN`, and then stores the `long` into an `int`, **truncating** to the low
32 bits. Both steps are modeled in `Scanner::scan_i32`. Cases where that flips
the branch away from the intuitive answer:

| stdin | `strtol` result | stored `int` | branch | stdout |
| --- | --- | --- | --- | --- |
| `2147483648` | 2147483648 | -2147483648 | `good()` | `string\n` |
| `-2147483649` | -2147483649 | 2147483647 | `good()` | `string\n` |
| `4294967296` | 4294967296 | **0** | `bad()` | `\n` |
| `99999999999999999999` | `LONG_MAX` | -1 | `good()` | `string\n` |
| `-99999999999999999999` | `LONG_MIN` | **0** | `bad()` | `\n` |

The last row is the trap: an enormous negative input takes the *same* branch as
`0`, because `LONG_MIN`'s low 32 bits are all zero. A translation that clamped
to `i32::MIN`, or that returned an error on overflow, would print `string\n`
here and differ from the C.

### 4. `%d` crosses newlines — no mismatch, pinned by test

`scanf` skips leading whitespace including `\n`, `\r`, `\t`, `\v` and `\f`, so
`"\n\n\n\n5"` converts to 5 and reaches `good()`. An `fgets`-based reader would
stop at the first newline, convert nothing, and reach `bad()` instead.
`percent_d_scans_across_newlines` locks this in.

On a matching failure the offending byte is pushed back and `x` is left
untouched at `0` (`"abc"`, `"-"`, `"+"`, `"--1"`, `".5"`, `"0x10"`, `"\xff\xfe"`
all take `bad()`), which `scanf_matching_failures_leave_x_at_zero` covers.

## Test suite

`tests/differential.rs` — 14 tests. Each spawns **both** binaries as
subprocesses, writes the same bytes to stdin, and asserts stdout, stderr and
exit status all match. Expectations come from running the C binary, never from
hardcoded strings, so the C stays authoritative. The C program is built via
CMake on first use, so `cargo test` works from a clean checkout. Nothing is
`#[ignore]`d.

Input classes covered: empty input; a single item on each branch; every `scanf`
matching-failure early-out; whitespace handling across newlines; only-the-first-
conversion-is-read; `INT`/`LONG`/`ULONG` boundaries and saturation; powers of
ten and runs of nines from 1 to 39 digits; 100 KB of leading whitespace and
50 000 leading zeros; SIGPIPE; every input of length ≤ 2 over the 19 bytes the
parser branches on; 400 deterministic pseudorandom inputs.

Beyond the committed suite, an offline sweep of ~6 000 further inputs —
exhaustive length ≤ 4 over `0-+ \n9a`, plus 3 000 random byte strings and long
digit strings around the 32- and 64-bit boundaries — found no mismatches.

## Status

Both programs build clean. `cargo test` passes in both the `dev` and `release`
profiles, 14/14, none skipped. `c_src/` is unmodified apart from the `build/`
directory the prescribed build command creates.
