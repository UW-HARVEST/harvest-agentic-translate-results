# Differential verification of the C → Rust translation

Reference: `c_src/src/main.c` (never modified). Tests: `translation/tests/differential.rs`.

## What the C actually is

```c
void driver(int x) {
    for (int i = 0, j = 0; i < x; i++, j += 2) printf("%d %d\n", i, j);
}
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

The whole branch space is: (1) whether `scanf` assigns at all, (2) what value it
assigns after truncation to `int`, (3) the `i < x` guard — zero / one / many /
`INT_MAX` iterations, and (4) the `j += 2` overflow deep inside the loop.
`main` has a single exit path, so the status is always 0 unless a signal kills it.

## How both programs are built and run

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                  # -> translation/target/release/driver
```

`cmake ..` with no `CMAKE_BUILD_TYPE` leaves `CMAKE_C_FLAGS` empty, so the
reference binary is **unoptimized**. That matters for the overflow case below;
the disassembly of `driver` is a plain `addl $0x2,-0x8(%rbp)` on a 32-bit stack
slot with a signed `jl` guard, i.e. two's-complement wraparound.

The tests drive both binaries as subprocesses over a stdin pipe and compare
stdout bytes, stderr bytes, and the exit status (including the terminating
signal). Nothing is loaded as a library.

## Mismatches found and fixed

### 1. `SIGPIPE`: C died from the signal, Rust exited 0

The only true behavioural divergence found.

```
$ printf '100000000\n' | c_src/build/driver | head -c 10 >/dev/null   # status 141 (SIGPIPE)
$ printf '100000000\n' | translation/.../driver | head -c 10 >/dev/null # status 0
```

Cause: the Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. `writeln!` then returns `EPIPE`, the translation discarded it with
`let _ = ...`, and the process ran to a normal `return 0`. The C keeps the
default disposition, so the first `printf` to a closed stdout kills it with
signal 13.

Fix (`translation/src/main.rs`): reset the disposition to `SIG_DFL` at the top
of `main` via an `extern "C" signal(13, 0)` call. Covered by
`closed_stdout_kills_both_with_sigpipe`, which also asserts the C side really
does die from signal 13 so the test cannot pass vacuously.

### 2. Test-harness defect: buffering `INT_MAX` iterations exhausted memory

Not a translation bug, but it hid the case. `-2147483649` truncates to
`+2147483647`, so the loop runs `INT_MAX` times and emits about 30 GB. The
initial `assert_same` buffered the whole of both streams and the test binary
aborted with `memory allocation of 8589934592 bytes failed`.

Fix: `assert_same_stdout_prefix` compares a bounded 1 MiB prefix (and asserts
both sides actually produced the full prefix, so it cannot degenerate into
comparing two empty buffers). The full-stream comparison is still used for
every input whose output is bounded.

### 3. Test-harness defect: the wrap search dominated the debug run

The first version of the overflow test scanned for a marker with
`windows().position()` over ~23 GB, which pushed plain `cargo test` to 9m40s.
Replaced with a closed-form byte offset (`output_bytes_before`, self-checked
against a brute-force count over the first 20 000 lines), so the streaming
comparison bottoms out in `memcmp`. Debug run is now 3m47s, release 2m39s.

## Differences investigated and confirmed to already match

### `scanf("%d")` accumulator width and truncation

glibc converts `%d` through a `long`-wide accumulator that saturates at
`LONG_MAX`/`LONG_MIN`, then stores the result **truncated** to `int`. This is
observable and gives some surprising counts, all of which the translation
already reproduced:

| input | stored `x` | output |
|---|---|---|
| `2147483648` | `-2147483648` | none |
| `4294967296` | `0` | none |
| `4294967297` | `1` | 1 line |
| `4294967300` | `4` | 4 lines |
| `-4294967293` | `3` | 3 lines |
| `-2147483649` | `2147483647` | `INT_MAX` lines |
| `99999999999999999999` | `-1` (saturates to `LONG_MAX`, truncates) | none |
| `-9223372036854775809` | `0` (saturates to `LONG_MIN`, truncates) | none |

The Rust `scan_i32` accumulates saturating in `i64` and then casts with `as
i32`, which is the same two-step. Covered by
`truncation_to_int_wraps_into_small_positive_counts`, `long_range_saturation`,
`int_max_and_just_past_it`, `absurdly_long_digit_runs`.

### `j += 2` signed overflow at i == 1073741824

Reached only after ~1.07e9 iterations, so it needs a real billion-line run.
Verified directly — both streams are byte-identical from offset 0 through the
wrap, and the line at the analytically predicted offset is:

```
1073741823 2147483646
1073741824 -2147483648      <- j wraps
1073741825 -2147483646
```

The C's unoptimized `addl` wraps; the Rust's `wrapping_add` matches. Covered by
`j_overflows_int_at_one_billion_iterations` (~90–130 s, not `#[ignore]`d).
`i++` is a signed increment too, but `i` can never reach `INT_MAX` because the
guard `i < x` fails first.

### `%d` skips whitespace across newlines

`%d` (unlike `fgets`) skips *all* leading whitespace including `\n`, `\r`, `\t`,
`\v`, `\f`. `\n\n\n4` yields 4. Already correct; the Rust whitespace set matches
`isspace` in the C locale. Covered by
`leading_whitespace_is_skipped_across_newlines`.

### Matching failure leaves `x` at its initializer

On a matching failure `scanf` stores nothing, so `x` keeps the `= 0` from its
declaration and the loop body never runs — the same visible result as a real
`0`. Inputs verified: `abc`, `-`, `+`, `-a`, `.`, `.5`, `--5`, `-+5`, `- 5`,
`-\n5`, `0x10` (parses the `0`, stops at `x`), `,5`, `_5`, `"5"`. Covered by
`matching_failure_leaves_x_at_zero`.

### Only one conversion runs

Everything after the first number is never read: `3 99`, `3abc`, `3\n7\n`,
`3,4`, `3.9` all print 3 lines, and a 200 KB junk tail changes nothing.
Covered by `trailing_junk_after_the_number_is_ignored` and
`huge_trailing_payload_is_never_read`.

### Non-UTF-8, NUL and non-pipe stdin

`\xff5`, `\x005`, `\xc3\x28 5`, `\x80\x81\x82`, stdin as `/dev/null`, and stdin
as a *directory* fd (where `read` fails with `EISDIR` rather than returning 0)
all behave identically — no output, exit 0. Covered by
`non_utf8_and_nul_bytes`, `stdin_is_dev_null`, `stdin_is_a_directory`.

### stdout buffer size

glibc sizes stdout's buffer from `st_blksize`, which is **4096** here for pipes,
files and terminals (`BUFSIZ` is 8192 but unused for this). Rust's
`BufWriter::new` defaults to 8192. Changed to
`BufWriter::with_capacity(4096, ...)` so the `write()` boundaries line up. This
is invisible when the reader drains stdout, but it decides how many bytes make
it out before the `SIGPIPE` death in fix #1.

Not replicated: C's stdout is *line*-buffered when it is a TTY. This changes
only flush timing, never the bytes produced, and it is unobservable under the
pipe-based comparison used for grading.

## Result

26 tests, none `#[ignore]`d, disabled or skipped.

```
cargo test --release   # 26 passed, 0 failed, 0 ignored (2m39s)
cargo test             # 26 passed, 0 failed, 0 ignored (3m47s)
```
