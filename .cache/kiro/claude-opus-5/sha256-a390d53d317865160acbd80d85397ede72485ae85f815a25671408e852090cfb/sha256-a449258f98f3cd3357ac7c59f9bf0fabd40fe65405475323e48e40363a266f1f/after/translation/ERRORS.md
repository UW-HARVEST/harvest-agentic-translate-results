# ERRORS.md — differential verification log

Comparison of `c_src/src/main.c` (ground truth) against `translation/src/main.rs`,
by running both executables on the same stdin bytes and diffing stdout, stderr
and exit status.

## Build and run commands

| | command |
|---|---|
| C build | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` |
| C run | `c_src/build/driver` (reads stdin) |
| Rust build | `cd translation && cargo build --release` |
| Rust run | `translation/target/release/driver` (reads stdin) |
| Tests | `cd translation && cargo test` |

## Result

**No mismatches were found.** Every input class below produced byte-identical
stdout, byte-identical stderr (both always empty) and exit status 0 from both
programs. The Rust source required no corrections; nothing in `c_src/` was
modified.

Because there were no mismatches to record, the rest of this file documents the
behaviours that *could* have diverged, the input that probes each one, and the
evidence that they do not. These are the places a naive translation of this
program goes wrong.

## Branch inventory of the C program

Every branch in the C source, and the test that reaches it:

| C location | branch | reaching input | test |
|---|---|---|---|
| `main` read loop | `i < 100` false — array filled | 100+ valid integers | `exactly_the_maximum_the_code_handles`, `input_longer_than_the_array_is_truncated_at_100` |
| `main` read loop | `scanf(...) != 1` via EOF | empty / whitespace-only input | `empty_input_produces_no_output`, `whitespace_only_input_is_an_immediate_eof` |
| `main` read loop | `scanf(...) != 1` via matching failure | `abc`, `-`, `.`, `\0`, byte `0x0e` | `matching_failure_on_the_first_item`, `sign_without_digits_is_a_matching_failure`, `nul_bytes_terminate_the_read_loop` |
| `main` read loop | `break` at every intermediate `i` | `1 .. n STOP 999` for n = 0..101 | `every_length_terminated_by_a_matching_failure` |
| `fma_array` loop | zero iterations (`len == 0`) | empty input | `empty_input_produces_no_output` |
| `fma_array` loop | one or more iterations | any valid integer | `single_item`, `every_length_from_zero_through_the_maximum` |
| `driver` print loop | zero iterations | empty input | `empty_input_produces_no_output` |
| `driver` print loop | one or more iterations | any valid integer | `single_item`, and most others |
| `main` | `return 0` (the only exit) | all inputs | every test asserts the exit status |

`len` is `i`, which is only ever in `0..=100`, so no negative-length path
exists. There is no error path that writes to stderr and no path that returns
a non-zero status: the C program always exits 0, and the tests assert that
rather than assuming it.

## Behaviours that had to be reproduced exactly

### 1. `scanf("%d", ...)` reads across newlines

`%d` skips *any* run of whitespace before the digits, so a newline is not a
record separator the way it would be with `fgets`. `1\n2\n3`, `1\t2\r\n3` and
`1\x0b2\x0c3` all yield three items. The Rust `Scanner::scan_int` skips the
full C `isspace` set (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`) before the sign, and
byte `0x0e` is deliberately *not* in that set.

Probed by `scanf_reads_across_newlines_and_all_whitespace` and
`whitespace_only_input_is_an_immediate_eof`.

### 2. Conversion overflow: saturate at `long`, then truncate to `int`

glibc's `%d` converts with `strtol` and stores the result into an `int`. Two
separate effects stack, and both are observable:

* A value that fits in `long` but not in `int` is **truncated** (low 32 bits).
  `2147483648` → `-2147483648`; `4294967296` → `0`; `4294967297` → `1`.
* A value that overflows `long` **saturates** at `LONG_MAX` / `LONG_MIN` first
  and is only then truncated. `99999999999999999999` → `LONG_MAX` → `-1`;
  `-99999999999999999999` → `LONG_MIN` → `0`.

A translation that used Rust's `str::parse::<i32>()` (which errors on overflow)
would have broken out of the read loop here and printed nothing, and a
translation that saturated directly at the `i32` limits would have printed
`2147483647` / `-2147483648` instead. The Rust accumulates into `i64`, latches
a `saturated` flag on overflow, then does `value as i32`.

Probed by `conversion_overflow_truncates_and_saturates_like_the_c_does`,
including 1000-digit inputs in both signs.

### 3. A sign with no digit after it is a *matching failure*, not zero

`-`, `+`, `-a`, `- 5`, `--1` and `+-1` all make `scanf` return 0, so the loop
breaks and nothing is printed. Reproduced by requiring at least one digit after
the optional sign, and by pushing the offending byte back (C's `ungetc`) rather
than consuming it.

Probed by `sign_without_digits_is_a_matching_failure`.

### 4. Partial conversions stop at the first non-digit

`5x` yields one item, `3.14` yields `3`, `0x10` yields `0` (stopping at `x`),
`1e5` yields `1`, `1_2` yields `1`. `%d` is decimal-only — there is no hex or
float handling.

Probed by `matching_failure_part_way_through`.

### 5. Aliased `fma_array` collapses to `v * v + v`

The only call is `fma_array(out, out, out, out, len)`, so `mul1`, `mul2`, `add`
and `out` are the same buffer. Within one iteration all three reads at index
`i` happen before the write at index `i`, so each element becomes
`v * v + v` — the aliasing does not change the per-element result, and the
`const` qualifiers on the parameters do not imply no-alias. The Rust models
this directly instead of taking four separate slices.

### 6. Signed overflow wraps two's-complement

`out[i] = v * v + v` overflows for most inputs (`46341 * 46341` already does),
which is undefined behaviour in C. The compiled C wraps, so the Rust uses
`wrapping_mul` / `wrapping_add`. Verified that the C binary's output is
unchanged at `-O0`, `-O1`, `-O2`, `-O3` and `-Ofast`, so the observed wrapping
is not an artefact of the unoptimised CMake default build.

Using plain `*` and `+` in Rust would additionally have *panicked* in a debug
build (exit 101 with a stderr message) rather than printing a wrapped value.
Both the debug and release Rust binaries were tested and neither panics.

Probed by `arithmetic_around_the_overflow_boundary` and `int_boundaries`.

### 7. Only the first 100 items are read at all

The loop guard `i < 100` stops the reads; the rest of stdin is never consumed.
So `1 .. 150` prints exactly 100 lines, and garbage placed after the hundredth
item is invisible.

Probed by `input_longer_than_the_array_is_truncated_at_100`.

### 8. Output formatting

`printf("%d\n", ...)` — one value per line, no field width, no padding, and a
trailing newline after the last value. Output is empty (zero bytes, not a bare
newline) when `len == 0`.

### 9. `int data[100]` is uninitialised in C

Only `data[0..i]` is ever read, and each of those was written by a successful
`scanf`, so the uninitialised bytes are never observable. Zero-initialising the
Rust array is therefore safe.

## Stream conditions also checked

Compared outside the cargo suite, since they depend on how the shell wires up
the file descriptors:

| condition | C | Rust |
|---|---|---|
| stdout redirected to `/dev/full` | exit 0, empty stderr | same |
| stdout file descriptor closed (`>&-`) | exit 0, empty stderr | same |
| stdout pipe closed early by the reader (`\| head -c 1`) | exit 0 | same |
| stdin closed (`<&-`) | exit 0, no output | same |
| stdin is a directory | exit 0, no output | same |

Notably neither program dies on `SIGPIPE` in these cases, so no signal-vs-exit
status divergence exists.

## Coverage summary

* 23 tests in `translation/tests/differential.rs`, all enabled — none
  `#[ignore]`d, skipped or disabled.
* Both `cargo test` (debug) and `cargo test --release` pass.
* Exhaustive over length: every `len` from 0 to 101, terminated both by EOF and
  by a matching failure.
* Randomized inside the suite: 300 token-mix trials, 200 raw-byte trials, 200
  numeric-alphabet trials, all from fixed seeds so a failure is reproducible.
* Additional out-of-band fuzzing: 23,000 trials across two seeds mixing valid
  integers, overflow literals, whitespace variants, invalid tokens, NUL bytes
  and fully random byte strings — zero mismatches.
